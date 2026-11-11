# MCP surface for Ozone-Studio (2026-11-11) — edit-only, `node --check` only, NOT RUN

## 1. Question and answer
*Can Claude Code / ZCode, as MCP clients, run a walk, link files, read call graphs and use Ozone-Studio fully?*
Before today: **no.** `tools/ozone-shared-context/server.js` is coordination only (presence, claims, notes, `task_create`,
`tasks_list`, search bridge). Now: **yes, through a sibling server** `tools/ozone-mcp/server.mjs` (registered in `.mcp.json`).
Chosen over extending shared-context because (a) shared-context deliberately stays local/stdio and reads the host's internal
token from the data dir (a same-user shortcut a third party must not use), (b) the two have different trust postures, and
(c) one shared state is kept anyway: both servers read/write the same `OZONE_CONTEXT_DIR` (`notes.jsonl`, append-only).

## 2. Tool set (13) — each description starts `READ.` or `MUTATING.`
| tool | host route | kind |
|---|---|---|
| `ozone_health` | GET /health | read |
| `ozone_task_list` / `ozone_task_get` | POST /task/list, /task/get | read |
| `ozone_work_order` | GET /order/work/<task_id> | read |
| `ozone_run_status` | GET /orchestrate/status?run_token= | read |
| `ozone_callgraph_system` / `_live` / `_files` | GET /callgraph/system, /live, /files | read |
| `ozone_gaps` / `ozone_refinement` | GET /capability/gaps, /methodology/refinement | read |
| `ozone_marketplace_view` | GET /marketplace/overview | read |
| `ozone_query` | POST /zsei/query — whitelist of 11 read variants, others refused | read |
| `ozone_orchestrate` | POST /orchestrate (`submit_only`) → `run_token` at once | **mutating** (spends model calls) |
| `ozone_link` (file / folder) | POST /pipeline/execute, pipeline 30 `Link` / `LinkFolder` | **mutating** |

Not exposed on purpose: marketplace `offers/decide` and any accept (operator-only), task cancel/resume, ZSEI writes, config,
pipeline registration, `/mcp/call`. Linking is how a file enters a project; graph creation then happens by itself
(`graph_on_link`, no prompt) and is read with `ozone_callgraph_files`.

## 3. Properties implemented
- **Auth**: the same ed25519 device-key challenge as `tools/e2e/lib.mjs`; `Authorization: Bearer` on every call; one re-auth on 401.
  Key and token are never printed or returned (errors pass through `scrub`). No use of the internal token or the data dir.
- **Identity**: every call carries an agent (`agent` arg else `OZONE_AGENT_NAME`, validated `[A-Za-z0-9._-]{1,64}`), sent as `x-ozone-agent`.
- **Read vs mutate** separated by name + description; every mutating call appends a note (agent, tool, hashes/lengths, run_token) — never the prompt or file text (`.ozone-context` is git-visible).
- **Long walks never block**: start returns a `run_token`; poll `ozone_run_status`, then `task_get` / `work_order` / `callgraph_live`.
- **No size refusals**: any result can be read in chunks (`offset`/`length` → `{chunk, offset, next_offset, total_chars}`); omit = whole.
- **Real errors only**: HTTP ≥400 or `success:false` is thrown with the host's own message; nothing becomes an empty success.
- **Scope binding explicit**: `workspace_id` / `project_id` are arguments to orchestrate/link (never guessed).
- `attached_files` is sent in the exact `AttachedFileSpec` shape (`file_path, mime_type, is_inline, content_preview`) because the host
  `unwrap_or_default()`s a malformed list (a wrong shape silently drops every attachment — src/lib.rs:1747).

## 4. Hardening review (findings → options for the operator)
| # | finding | status / option |
|---|---|---|
| H1 | **Auth/identity**: all agents on one machine share one device key, so the host sees one user; `agent` is self-declared, so one agent can claim to be another (shared-context claims are advisory by design). | Done: header + note attribution. **Option A** keep (loopback, trusted agents). **Option B** one device key per agent (`OZONE_AGENT_KEY_PATH`, needs each key paired once) — real separation in host logs. |
| H2 | **Tool-arg injection**: ids are integers (`Number.isSafeInteger`), run/trace tokens are regex-checked, query strings are `encodeURIComponent`ed, host is fixed by env. | Done. |
| H3 | **SSRF**: host comes only from `OZONE_HOST`; non-loopback needs `OZONE_MCP_ALLOW_REMOTE_HOST=1`; no tool takes a URL. | Done. Using the EU host from a laptop is a deliberate flag. |
| H4 | **Link paths**: absolute, `realpath` (symlinks resolved), must sit inside `OZONE_MCP_LINK_ROOTS` (default = server cwd), NUL rejected, file vs folder checked; same check on `attached_files`. | Done in the MCP. **Gap**: the host itself accepts any path from a direct HTTP caller (`/orchestrate` `attached_files`, pipeline 30) — the MCP can only restrict its own callers. **Option**: a host-side allow-list of linkable roots (operator-set), default off-limits dirs (`~/.ssh`, the data dir, `/etc`). |
| H5 | **Concurrent agents**: shared-context `state.json` is read-modify-write without a lock/atomic rename (presence + claims). Two agents heartbeating at the same instant can lose an update; notes are append-only and safe. | Not changed (not this task's file). **Option**: write state via temp file + rename and a lock file; or move presence/claims into the host. |
| H6 | **Claim protocol vs the MCP**: `ozone_link`/`ozone_orchestrate` do not take claims (they do not edit repo files). Linked project files are not coordinated. | By design; **option**: refuse `ozone_link` on a path another live agent has claimed. |
| H7 | **Data exposure**: `ozone_query GetContainerContent` returns stored content (private to the operator's host). It is a read of the operator's own data by the operator's own agents; `.ozone-context` never receives it. | Accepted. **Option**: narrow the whitelist to metadata variants. |
| H8 | **Prompt injection via results**: tool results contain user/ingested text. Clients must treat results as data. | Documented in the tool descriptions' READ/MUTATING stance; not enforceable server-side. |
| H9 | **Usage gate**: shared-context applies the host's daily MCP limit (`/mcp/usage`); `ozone-mcp` does not (no fixed caps by doctrine; the host's own model-call guards still apply). | **Option**: count calls via `/mcp/usage` for visibility only. |
| H10 | **No self-registration**: shared-context registers itself in the host tool registry using the internal token; `ozone-mcp` does not (that needs the data-dir token). | **Option**: register through an operator-run command with a session instead. |
| H11 | **Cost**: `ozone_orchestrate` spends the free-tier chain; any agent can start walks. | **Option**: require `OZONE_MCP_ALLOW_WALKS=1` per agent env (off = read-only agent). Not implemented — the operator rules switches default ON. |

## 5. Drafted test suite (not run) — `tools/third-party-verify/mcp/`
`harness.mjs` (stdio JSON-RPC client + three-agent launcher, throwaway shared context dir), `m1-coordination.mjs`
(presence/claims/note handoff across 3 agents, identity validation, refusal of non-read queries, chunked read),
`m2-walk.mjs` (opt-in: A starts a walk, B/C observe, completion gate: completed ⇒ no open item, awaiting ⇒ an open item;
live call graph; gaps before→after), `m3-link-modalities.mjs` (fixtures for text, code, image, audio, 3d obj/stl; link hardening
refusals; **link first → non-semantic graph per file from the ledger with no prompt → second agent sees the same rows → walk over
the same file REUSES the same `graph_id`/hash**). Ledger fields checked are the real ones in `graph_on_link.rs`
(`status graph_created|complete`, `graph_id`, `hash`, `structural_done`, `trigger`, `created_by`). Modalities without a
fixture (and the 8 sensor modalities the readers record as `unreadable`) are SKIPPED, not faked.

## 6. Unverified
- Nothing was run: servers and scripts are `node --check` only. Response shapes (`/task/get` returns `Option<TaskInfo>` i.e. `null` for an unknown id; `/orchestrate/status` fields) come from handlers, not live payloads.
- Pipeline-30 result shape and whether `/pipeline/execute` demands a project container id (`zsei_parent_id`) for the graph to attach: pass `OZONE_ZSEI_PARENT_ID` in `m3`.
- `ozone-shared-context` heartbeat/claim refusal shapes were read from `server.js` (`ok:false, conflicts`).
- `consciousness_enabled` is omitted unless given, so the host default (true) applies; `tools/e2e` sends false.

## 7. Operator decisions needed
H1 (per-agent keys), H4 (host-side link allow-list), H5 (atomic state), H6/H9/H10/H11 as listed above.
