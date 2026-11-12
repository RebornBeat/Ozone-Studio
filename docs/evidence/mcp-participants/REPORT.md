# MCP agents as walk participants (2026-11-11) — edit-only, parse-only, NOT built, NOT run

Operator ruling: MCPs participate IN the prompt/walk. An MCP call can BE the prompt; the walk runs internally and INCLUDES the
agents that made it (and any it names); those agents are told about the walk through shared MCP context and are aware of the
walk they are making; a walk step can be handed to one of them. Builds on `docs/evidence/mcp-surface/REPORT.md` (13 tools).

## 1. What now exists

| piece | where | what it does |
|---|---|---|
| walk participants | `src/walk_feed.rs`, `src/grpc/mod.rs` (`/orchestrate`) | `OrchestrateRequest` gains `agent` + `participants`. The handler creates the walk's TRACE root itself (`with_trace(TraceCtx::new_root("orchestrate"), …)`; the orchestrator's own `child_or_root` joins it), so run_token ↔ trace_id exists from the first instant. `walk_participants.jsonl` (append-only): one `walk` row (run_token, trace_id, caller first, participants, scope) and `bind` rows (trace → task_id, written when the walk returns its task id). Both the blocking and `submit_only` paths record. |
| live feed | `src/walk_feed.rs`, boot in `grpc/mod.rs` | `spawn_collector()` subscribes to `OrchestrationEventHub` and appends every stage event to `walk_feed.jsonl` (monotonic `seq`; a lagging receiver writes a `lagged` row, never a silent gap). Never rotated/capped. Per-node graph events are NOT mirrored (voluminous; `/callgraph/*` serves graph state). |
| host routes | `grpc/mod.rs` | `GET /walk/feed?run_token=\|trace_id=\|task_id=&after=` → `{found, walk, rows, next}`; `GET /walk/mine?agent=`; `/orchestrate/status` now also returns `trace_id`, `caller`, `participants` (running and finished). `/walk/` and `/agent/` are in the strict-session PROTECTED list. |
| MCP side | `tools/ozone-mcp/server.mjs` | `ozone_orchestrate` takes `participants` and sends `agent`; announces the walk in the shared context dir (`walks/index.jsonl`, hashes + names only), starts a background feed loop (adaptive pacing, no fixed cap) and returns the participants. New tools: `ozone_walk_feed` (cursor; pulls host stage rows + status transitions + work-order item changes into ONE shared file `walks/<run_token>.jsonl`, so every participant reads identical rows; any participant's call advances it), `ozone_walk_mine`, `ozone_whoami`, `ozone_agent_requests`, `ozone_agent_report`, `ozone_agent_offer`. |
| agent provider | `src/agent_requests.rs`, `src/mcp.rs`, `src/orchestrator/tool_routing.rs`, `grpc/mod.rs` | New transport `agent` (`McpTransport::Agent`), endpoint `agent:<name>` or `agent:*` (any participant of the walk). A provider step that calls it creates a durable REQUEST (`agent_requests.jsonl`) and AWAITS the agent's report: no timeout, no fail-open, no delegate-and-move-on. Completed is final: a second report is refused. Request id is deterministic from (tool, task_id, step_index, input) so a resumed walk ADOPTS the earlier request and answer; a rerun is a new task → a new request. Report authorisation: the target agent, or for `agent:*` a participant of the request's walk. `POST /mcp/tools/register` accepts `agent` transport and validates the endpoint form; `call_provider_for_step` now also lists `agent:` endpoints. Routes `GET /agent/requests?agent=`, `POST /agent/report`. |
| hardening | see §3 | H1, H4, H5, H6, H10, H11 applied; H9 deliberately not. |

How an agent uses it: `ozone_agent_offer {capabilities:[…]}` once → blueprints/steps can name provider `agent:<agent>`;
`ozone_agent_requests` shows its pending steps; `ozone_agent_report {id,result}` completes the step. A caller starts a walk
with `ozone_orchestrate {prompt, participants:[…]}` and every participant follows it with `ozone_walk_feed {run_token, after}`.

## 2. Decisions taken (operator said "you decide")
- **Feed storage:** host-side jsonl + MCP shared file, not a ring buffer — a fixed buffer would be a cap (the existing monitor hub's 500-event ring cannot carry a long walk). Shared walk rows live in `.ozone-context/walks/` which is now git-ignored (stage summaries can hold prompt-derived text); git-visible notes carry hashes/ids only.
- **Early task binding:** `bind_task` runs when the walk RETURNS its task id (host handler). Binding at task creation needs one line in `src/orchestrator/stages.rs` (§4, hook 1) — that file belongs to another fork.
- **Agent identity is declared, bound per process:** the host cannot authenticate which agent a caller is when agents share one device key. ozone-mcp therefore binds identity to the process (one server = one agent) and supports per-agent keys; the report records the validated device id when a session token is sent.
- **shared-context strictness is opt-in** (`OZONE_CONTEXT_STRICT_AGENT=1`): one Claude Code session legitimately speaks through that server as several named forks (AGENTS.md protocol); forcing one name would break every fork's claims. ozone-mcp is always bound — forks calling it omit `agent`.

## 3. Hardening
| # | change |
|---|---|
| H1 | ozone-mcp: `agent` arg can only equal `OZONE_AGENT_NAME` (`OZONE_MCP_ALLOW_AGENT_OVERRIDE=1` to override deliberately); per-agent device key `OZONE_AGENT_KEY_PATH` or `~/.ozone-cli/agents/<name>/device-key.pem`, else the shared key (`ozone_whoami` says which; never prints a key). Needs each per-agent key paired once. shared-context: opt-in binding. |
| H4 | New `src/link_guard.rs`, applied in `/orchestrate` (`attached_files`) and `/pipeline/execute` (every `file_path`/`folder_path` in the input, nested too): host secrets (data dir, `config.toml*`, `.internal_token`, device keys, `~/.ssh`, `~/.gnupg`, `~/.aws`, `~/.ozone-cli`, gcloud config), `/etc`, `/proc`, `/sys`, `/dev` refused; `OZONE_LINK_ROOTS` (":"-separated) is an optional operator allow-list; symlinks resolved first; `OZONE_LINK_GUARD=off` disables (default ON). A security refusal about WHAT a path is, never about size. |
| H5 | shared-context `state.json`: atomic temp+rename write, and a pid-owned lock file around presence/claim/release read-modify-write (stale lock of a dead pid is taken over; a live owner is waited for — no fixed cap). |
| H6 | `ozone_link` refuses a path inside a file/dir claimed by another LIVE agent (reads `state.json`). |
| H10 | `node tools/ozone-mcp/server.mjs --register` — operator-run, uses a real session (not the data-dir token), registers one stdio row `ozone-mcp` with the tool names as capabilities. |
| H11 | `OZONE_MCP_READ_ONLY=1` makes an agent read-only (mutating tools refused). Default off. |
| H9 | NOT wired: `/mcp/usage` enforces a daily limit per agent, which is quota gating (operator doctrine: no quota sharing/gating). |

## 4. Follow-up edits in files owned by another fork (NOT made)
1. `src/orchestrator/stages.rs:3009` (where `state.task_id = Some(task_id)` is set): add `if let Some(t) = crate::trace::current_trace() { crate::walk_feed::bind_task(&t.trace_id, task_id); }` so participants resolve the walk by task id DURING the walk (today: when it returns). Likewise `run_checkpoint.rs:219/285` on resume (the resumed walk has a new trace id: bind it to the same task id so the feed follows the resume).
2. Work-order / AMT-expansion / gap events are not broadcast by the host; the MCP derives them by diffing `/order/work/<task_id>` and the status. Push events would need an event hub emit in `work_order.rs` / `walk_expansion.rs` (`expand_walk`) and `task/mod.rs` (`complete_item`, `graft_item`, `release_awaiting`/awaiting_children) that `walk_feed::append_row` can record (`kind: "work_item" | "amt_expansion" | "awaiting_children"`). `append_row` is `pub` and ready.
3. Agent-provider steps are only chosen when a blueprint step names provider `agent:<name>`. Nothing yet instructs the planner that registered agent providers exist; the provider list given to blueprint drafting (`tool_binding` / methodology `requires`) already includes every registry row, so an offered agent shows up like any other provider — verify on a real walk.

## 5. Risks / unverified (all code parse-only: rustfmt, `node --check`)
- Compiler-class items to check first: `tokio::spawn(crate::trace::with_trace(..., async move {..}))` Send bound; `watch::Ref` is never held across an await in `agent_requests::request_and_await` (it is dropped at the end of the `if let`); `rx.borrow().clone()` shape; `McpTransport::Agent` match arm in `McpRegistry::invoke` (only match on the enum in the repo); `req.project_id`/`req.workspace_id` use after the pipeline input was built.
- `POST /mcp/call` holds the runtime and registry READ guards for the whole call; an agent-provider tool called through that route therefore holds them until the agent reports (a fair RwLock queues writers behind it). The WALK does not use that route (`call_global` holds no guard); only the test (`m4` Part A) and direct callers do. Fix when verified: drop the guards before `invoke`.
- `agent_requests.jsonl` / `walk_feed.jsonl` / `walk_participants.jsonl` are read in full per request (no index). Fine for today's volumes; an index by trace is the follow-up if they grow large.
- Per-node graph events are not in the feed by design; `lagged` rows mark dropped stage events.
- Response shapes read by `advanceWalk` (`/orchestrate/status` fields, `/order/work` tree) are from handlers, not live payloads; `flattenItems` reads the tree defensively.
- Nothing here was run; no EU change.

## 6. Tests (drafted, not run) — `tools/third-party-verify/mcp/`
`m4-participants.mjs` (new): Part A needs no model calls — offer → agent-provider call via `/mcp/call` waits → request visible only to its target → wrong agent's report refused → target's report completes it with exactly that result → second report refused. Part B (opt-in): A starts a walk naming B and C; B and C discover it, follow it via `ozone_walk_feed`, read identical rows, stage rows carry the walk's trace id, task binding present. `m1` gained I2/I3 (identity binding, `ozone_whoami`).
