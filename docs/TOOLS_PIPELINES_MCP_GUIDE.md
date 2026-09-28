# Tools, Pipelines, and Bridges — the three-tier taxonomy, the gamedev-all-in-one integration, and the ordered shared context

> Seventh doctrine doc. CONTRACTS = algorithms; ZERO_SHOT_CALL_REGISTRY =
> model calls; BUILDER_REGISTRY = builders; TOP_DOWN_REVIEW_GUIDE = context
> flow to gates; CONTEXT_REGISTRY = context sources/carriers;
> ZERO_SHOT_EXPANSION_GUIDE = intelligence expansion; GRAPH_RELATIONSHIP_
> REGISTRY = every node/edge; **this doc = the integration taxonomy: what a
> PIPELINE is vs what a TOOL is vs what a BRIDGE is, where
> gamedev-all-in-one-mcp lands, the bridge-expansion contract, and the
> ordered shared-context protocol (Ozone-Studio as the review/order layer
> for every actor).**

---

## 1. The three-tier taxonomy

The operator's distinction, formalized and refined: **pipelines are
internal, tools are external — and bridges are the long-lived form of
both.** All three are registered by kind + name (CONTRACTS doctrine);
what differs is WHERE the capability lives and WHO dispatches it.

| Tier | Lives where | Shape | Dispatch | Example |
|---|---|---|---|---|
| **Pipeline** | Inside Ozone-Studio (assets/pipelines/) | independent compiled crate, `{data, context}` JSON envelope, one-shot or serve-mode | `PipelineExecutor` → registry gate → binary search or remote dispatch | prompt (9), text (100), decision_gate (39) |
| **Tool** | External process / server (any language) | MCP tool definition (name + zod/JSON schema + handler), registered into the host tool registry | `/mcp/call` → `McpCall` → metered, gated, rippled; `install_global`/`call_global` for in-process stages | ozone-shared-context (16 tools), gamedev's 67 tools |
| **Bridge** | External long-lived process, connected | announces at `/pipelines/register` (id, name, execute URL), re-announces on heartbeat, answers `POST <execute_url>` | RemotePipelines: connect-first, spawn-fallback; discovery = detection, transport = HTTP long-poll or TCP | gamedev's Roblox/Unity/Unreal/Blender connectors |

**The key structural fact** (verified by direct read of both codebases):
gamedev's connector pattern — detect executable → check bridge port →
handshake → long-poll/TCP command loop with auto-reconnect — is
**structurally the same pattern** as Ozone's remote-pipeline connect model
(`src/pipeline/remote.rs`: `register()` is the heartbeat, re-registration
refreshes `registered_at`, `execute()` dispatches to live connections
first, never spawning over one). Ozone already has the general mechanism;
gamedev's four engine connectors are four more instances of it.

### The precise definitions

**Pipeline** (internal executable unit):
- Own crate, own `[workspace]` root, own build. Compiled binary — never source.
- Wire contract: host sends full `{"data": {...}, "context": {...}}`
  envelope; pipeline answers with its output shape. One-shot
  (`--input <json>`, one JSON on stdout) or serve mode (`--serve`,
  self-registers, answers `POST /execute`).
- Executable ONLY through the registry gate (id must be in
  `PipelineRegistry.blueprints`). 82 loaded today.
- Zero-shot model calls from pipelines are mechanical transforms; the
  intelligence calls live at the orchestrator level (established
  doctrine — pipelines have no host session auth and must not call
  pipeline 9 directly; the SubprocessExecutor exception is scoped to
  text/code/math extraction sites).

**Tool** (external capability):
- A named operation with a schema, registered into the host's MCP tool
  registry (`/mcp/tools/register`), invocable by any agent (human UI,
  Claude Code, ZCode, another MCP server) through ONE standardized call:
  `/mcp/call` with `{tool, agent, input, context}`.
- Every call is **metered** (per-agent/day/tool usage ledger,
  `OZONE_MCP_DAILY_LIMIT` gate), **gated** (jurisdiction runs on tool
  input like any other input), and **rippling** (the call emits a scoped
  GraphEvent).
- A tool NEVER bypasses review: the standardized call is the review
  surface. This is what makes the ordering protocol (§3) possible.

**Bridge** (external long-lived connector):
- A bridge IS a tool server's transport half: it holds a connection to
  some external system (game engine, editor, device) and exposes that
  system's operations as tools.
- Registration: `POST /pipelines/register` (connect model) — the same
  mechanism a serve-mode pipeline uses. A bridge announces, heartbeats,
  and answers execute calls. Ozone treats it as a remote pipeline whose
  binary happens to live in another process.
- Bridge state is **graph state** (§2): connection, capabilities, health,
  and every operation executed through a bridge become containers and
  ripple events in the living graph.

### Why the distinction matters

- **Pipelines** are versioned, built, and deployed WITH Ozone — they ship
  in `assets/`, they are Ozone's own computation.
- **Tools** are capabilities Ozone CONSUMES or EXPOSES — they extend what
  the system can do without changing its code.
- **Bridges** are how external systems PARTICIPATE in the graph — their
  operations flow through the same metering/gating/rippling as everything
  else, and their state lives in the same fabric.

---

## 2. gamedev-all-in-one-mcp — what it is and where it lands

The repo sits at `gamedev-all-in-one-mcp/` (repo root, sibling to
`tools/`). Direct read of its structure (5,001 lines TS):

| Layer | What it actually is | Ozone counterpart |
|---|---|---|
| `src/server/create-server.ts` | MCP server, 12 tool modules, 67 tools registered by kind | `src/mcp.rs` McpRegistry + `/mcp/tools/register` |
| `src/connectors/{luau,unity,unreal,blender}/` | 4 engine bridges: detect → port check → handshake → command loop → auto-reconnect | `src/pipeline/remote.rs` connect model (identical shape) |
| `src/web/mcp-http.ts` | Streamable HTTP MCP (stateful sessions, client registry, bearer auth, idle pruning) | Ozone's HTTP surface; the client registry ↔ `.ozone-context/state.json` presence |
| `src/web/server.ts` + `dashboard.html` | Fastify dashboard :3100, SSE activity feed, glassmorphism dark UI | Ozone UI (Electron) + monitor feed — the dashboard panels are the same UX family the UI fork plan builds |
| `src/web/llm-proxy.ts` + `control-plane/providers.ts` | AI console: multi-provider chat with auto tool execution | Ozone's own model layer (pipeline 9 + fallback chain) — this part does NOT port; Ozone's model layer replaces it |
| `src/web/jobs.ts` + `activity.ts` | Job store + activity hub (in-memory + jsonl) | GraphEventHub + task system |
| `src/project/manifest.ts`, `tools/foundation.ts` | project manifest + doctor/capabilities | Workspace/Project containers + pipeline registry |
| `src/tools/*-physics.ts` | 20 physics tools across 4 engines | external capabilities → Tools tier |

**Verdict: gamedev-all-in-one is a TOOL SERVER + BRIDGE HUB.** Its 67
tools are the Tool tier; its 4 engine connectors are the Bridge tier; its
dashboard/AI-console/job-store layers are respectively redundant with,
replaced by, or mappable onto Ozone's existing surfaces.

### The integration shape (conversion, per operator directive)

1. **Bridges → Ozone's connect model.** Each engine connector ports to an
   Ozone-side registration: at boot (or on demand), the bridge process
   announces at `/pipelines/register` with a synthetic pipeline id
   (e.g. 200-series "bridge" ids: 200 roblox, 201 unity, 202 unreal, 203
   blender), heartbeats via re-registration, and answers execute calls.
   Detection/diagnosis (`doctor`) maps to the existing device/pipeline
   status surface. **Bridge expansion is then free**: any new engine is
   one more connector module registering the same way — no host code
   changes, the registry gate and connect model are generic already.

2. **Bridge state → graph state.** Every bridge gets a real container
   under a new `/External/Bridges` root (or reuse `/External/`, which
   already exists as a structural root):
   - `container_type: Pipeline` (or a new `Bridge` variant — additive),
     keywords `[bridge, roblox, http:3002, connected|disconnected]`,
     `object_store_path` pointing at a bridge-state JSON (ports, last
     handshake, capability list, error history).
   - Connection/disconnection/tool-execution emit scoped `GraphEvent`s —
     the ripple wakes AMT expansion candidates exactly like every other
     graph write (`GraphRipple` route, `docs/AMT_EXPANSION.md`).
   - Every operation executed through a bridge is a `/mcp/call`: metered
     in the usage ledger, jurisdiction-gated, captured.

3. **Tools → `/mcp/tools/register`.** The 67 tools register into Ozone's
   tool registry (bulk import: gamedev's own
   `GET /api/clients`-style capability list → per-tool register calls).
   They stay executable from any MCP client directly OR through Ozone's
   metered `/mcp/call`.

4. **What does NOT port**: the AI console + llm-proxy (Ozone's pipeline-9
   + fallback chain replaces it), the dashboard's engine-status panel
   (Ozone UI's Devices/Monitor tabs replace it), the job store (Ozone's
   task system replaces it). The dashboard itself can retire once the
   Ozone UI surfaces the bridge panels (Batch F/Devices family).

---

## 3. The ordered shared context — Ozone-Studio as the review/order layer

**The operator's architecture, captured:**

> The shared context is for cross-sharing: Ozone-Studio manages it;
> other models and MCPs store into it. It is itself a WORKSPACE; each
> origin — claude-code, zcode, any MCP — is a PROJECT under it. Context
> is stored per-origin as it comes in, under a contract. And the most
> important part is ORDER: today, Claude shares insights and reads when
> it wants, ZCode the same — no order, no direct call, no Ozone review.
> From now on: **each call to the shared context is directed through
> Ozone-Studio. Ozone reviews the context, reviews its internal order
> (tracking movement across the graph), and responds with insights —
> progression, what was done, what was captured, what may have been
> missed — per the todo/state. The actors await that response.**

### The contract (what exists today vs what this adds)

**Exists today** (verified): `.ozone-context/state.json` (presence,
claims, 300s TTL) + `notes.jsonl` owned by the Node MCP server; every
note/decision/handoff/finding/claim mirrors into ZSEI as
`CoordinationEvent` containers under `/SharedContext` (root 8) with
scope keywords (`scope:global` / `ws:<id>` / `proj:<id>`, kind keywords);
`context_mirror.rs` fires on the same events; B9/B10/B11 read routes live
in the host; the coordination layer feeds scoped history into step
context (task 43).

**The gap** (operator-identified, real): writes are fire-and-forget.
There is no review step — an actor calls `note_add` and never hears
anything back but a note id. No ordering, no insight, no
captured/missed accounting.

**The design — one write path, one review response:**

```
Actor (claude-code / zcode / any MCP client)
   │  note_add / task_create / file_claim / ...
   ▼
ozone-shared-context server  ──(unchanged local state.json writes)──▶  state
   │
   └── mirror → /mcp/call  {tool: "context_mirror", agent, input, context}
                ▼
        Ozone-Studio  THE ORDER LAYER
        1. Jurisdiction gate (always first, unchanged)
        2. Review the event: kind, scope, files, agent identity
        3. Internal ordering:
           - CoordinationEvent container (as today)
           - methodology match: does this event touch a methodology
             domain? → link + load its rules for the response
           - AMT expansion: GraphRipple candidate (existing route) —
             does this event change a project's plan-under-standing?
           - capture check: what did THIS actor last do; what's open
             (claims, ReviewPending items, unresolved zero-shot)
        4. Respond with the ordered insight envelope (below)
                ▼
Actor receives: not just an ack, but the state-of-the-world reply
```

**The insight envelope** (what every mirror call returns):

```json
{
  "success": true,
  "note_id": 8123,
  "review": {
    "progression": "3rd handoff this session on proj:3; follows zcode's C1 landing",
    "captured": ["mirrored as CoordinationEvent 8123", "ripple emitted (proj:3)", "claim on X acknowledged"],
    "maybe_missed": ["2 ReviewPending reviews older than 24h", "unreleased claim on ui/src/... from 2 days ago"],
    "related": {"methodology": "method_16 host-ops", "amt_generation": 40059, "open_tasks": ["69", "70"]},
    "order": "queued behind nothing; safe to proceed"
  }
}
```

Every field is real data from the graph/task/capture stores — the same
no-fabrication rule as everything else. `maybe_missed` comes from the
consciousness review pass's own sources (S10/S11 + task store + claims);
`related` from methodology match + AMT lineage; `order` from the task
queue.

**Why this works with what exists**: `/mcp/call` is ALREADY the
standardized, metered, gated, rippled abstract call (task 45's
`install_global`/`call_global` proves in-process stages can issue them;
the HTTP path exists for external actors). The mirror path just stops
being a side-channel and becomes the reviewed front door.

### Workspace/project identity mapping

- The shared context IS a workspace: a real `Workspace` container
  (e.g. "Agent Coordination"), today's `/SharedContext` root 8 becomes
  its project child (or keeps root-8 id with the workspace as parent —
  one migration, additive).
- Each origin is a project-scoped identity: `proj:claude-code`,
  `proj:zcode`, `proj:gamedev-mcp` (or numeric projects with keyword
  aliases). Events carry their origin project scope; the B9 scope filter
  already reads these keywords exactly.
- Presence (`.ozone-context/state.json` sessions) maps 1:1 to the origin
  projects — B10 already serves it.

---

## 4. The bridge-expansion contract

Any external system joins Ozone through ONE pattern (this is the whole
contract — the gamedev engines are instances 1-4):

1. **Register**: `POST /pipelines/register` `{pipeline_id, name,
   execute_url}` — synthetic id in the bridge range (200-299 reserved).
2. **Heartbeat**: re-register on interval (existing semantics —
   re-registration IS the heartbeat).
3. **Execute**: answer `POST <execute_url>` with the `{data, context}`
   envelope; reply with the operation result.
4. **Graph state**: Ozone maintains the bridge's container (connection
   state, capabilities, health) under `/External/Bridges/`, ripples every
   state change, meters every operation through `/mcp/call`.
5. **Tools**: the bridge's operations register as MCP tools
   (`/mcp/tools/register`) so any agent can call them through the
   standardized surface.
6. **Review**: every bridge call passes the jurisdiction gate like every
   other input. Bridge failures are data (`success: false` + error),
   never crashes.

New engine = new connector module following 1-5. No host changes. This
is why gamedev's architecture is valuable as a donation: its connector
pattern (detect/handshake/command-loop/reconnect, per-engine plugin) is
exactly this contract, already proven across 4 engines with 3 transport
types (HTTP long-poll, TCP, and Luau runtime).

---

## 5. AMT expansion wiring

The shared-context and bridge events join AMT expansion through the
EXISTING route — no new expansion pathway:

- **GraphRipple** (`amt_loop.rs::spawn_graph_ripple_sync`): any scoped
  graph write (CoordinationEvent containers, bridge-state updates) emits
  a `GraphEvent`; the sync appends an expansion candidate for the
  affected project's AMT; the review loop consumes it identically to
  every other candidate (methodology guidance + fallback escalation).
- **Continuation** (fork lineage): an actor's session of work on a
  project IS a fork of that project's main AMT (the main/fork island
  model, `docs/AMT_EXPANSION.md`). Ordered handoffs become the lineage
  trail: each `handoff` note event = one generation boundary candidate.
- **Methodology rules for coordination** (operator: "wire this now with
  the AMT expansion to have rules created for this"): the meta-loop's
  gap detection applies — coordination events whose keyword signal
  matches zero methodologies become gaps; the meta-loop drafts real
  coordination methodologies (e.g. handoff hygiene, claim lifecycle,
  bridge-operation safety) through the existing draft/persist pipeline.
  First candidate domains are already implied by the events flowing:
  claim-release symmetry (the B11 one-way-mirror finding), handoff
  citation discipline, bridge-connection lifecycle.

---

## 6. UI/UX integration surface

The gamedev dashboard's genuinely-good parts map onto the UI fork plan
(docs/UI_UX_FORK_PLAN.md — Batches A/B complete, C 13/13 landed, wave-3/4
partially landed before the rate-limit cut):

| Gamedev surface | Ozone UI home |
|---|---|
| Engine Status panel (4 bridges, animated glow) | Devices tab / future Bridges panel (same real-state discipline: connection truth from bridge containers, never fabricated) |
| Event Log (SSE stream) | Monitor tab + Coordination feed (I1) — same feed, scoped |
| Client registry ("Connected Agents") | Coordination tab presence (B10/I2) |
| AI Console | none — Ozone's own chat IS the console; gamedev's llm-proxy retires |
| Tool Registry (67 tools, color-coded, live search) | Tools tab — register the 67 tools and it renders them for free |
| Jobs panel with review badges | Task system + `/task/update` review flow |

The UI fork plan's own doctrine (real data or honest empty state, never
fabricated) is the SAME standard gamedev's dashboard claims; the
difference is Ozone enforces it at the graph layer (containers are the
data), not just the component layer.

---

## 7. What this changes in the registries (when built)

Per the standing registry-update rule (§6 of ZERO_SHOT_EXPANSION_GUIDE):

1. CONTRACTS §2 (PipelineExecutor): document the bridge id range
   (200-299) and that bridges are remote pipelines whose binary lives in
   another process.
2. ZERO_SHOT_CALL_REGISTRY: bridge operations are NOT zero-shot model
   calls — they're tool calls; the registry gains a note that
   `/mcp/call` is the parallel surface for non-model capabilities.
3. CONTEXT_REGISTRY: new source S12 (bridge/coordination event stream →
   graph containers), carrier = direct container fetch for consumers.
4. BUILDER_REGISTRY: the order-layer review (§3) becomes a builder
   section (the insight-envelope assembly).
5. GRAPH_RELATIONSHIP_REGISTRY: `Bridge` container type + relation types
   (`ExposedBy`/`ExposedTo` for bridge→tools, `ConnectedVia` for
   bridge↔engine), additive.

---

## 8. Build order (dependency-first, no dates)

```
Phase 0 (no code): operator decisions
  ├─ approve the ordered-shared-context contract (§3) and the
  │  insight envelope shape
  ├─ approve bridge id range 200-299 + /External/Bridges root
  └─ approve which gamedev layers port vs retire (§2.4)

Phase 1 (host, small): the review seam
  ├─ /mcp/call gains the order-layer review: assemble the insight
  │  envelope from real stores (consciousness sources + task queue +
  │  methodology match + claims/ReviewPending aging)
  ├─ ozone-shared-context server.js: mirror calls route through
  │  /mcp/call (env-gated, fallback to direct mirror if host down)
  └─ claim-release symmetry fix (server.js file_release mirrors too —
     the B11 one-way finding, real lie-in-the-UI bug)

Phase 2 (bridge port): gamedev connectors → Ozone bridges
  ├─ one connector first (Roblox, the HTTP one — simplest transport)
  ├─ bridge container + state JSON + ripple on state change
  ├─ tools bulk-register into /mcp/tools
  └─ then Unity/Unreal/Blender (same module shape)

Phase 3 (methodology): coordination methodologies via meta-loop
  └─ let gap detection run against real coordination-event keywords

Phase 4 (UI): bridge panels + coordination origin views
  └─ Devices/Bridges panel, per-origin project views (§3 mapping)
```

Everything above is additive to the existing systems. Nothing existing
is replaced except gamedev's own redundant surfaces (§2.4).

---

## §9. Universal native task ordering (2026-09-27, operator architecture)

**The operator's vision, formalized:** blueprints already contain ordered
steps — steps ARE tasks — that is the native checklist/todo. What's
missing is the GLOBAL view: aggregate across ALL AMT trees (what's live,
what's paused, what's interrupted) into one universal task order, with
consciousness and meta-loop work registering on the SAME queue, and all
external work (shared-context actors, gamedev MCP operations) joining
through AMT expansion. One hierarchy maintains order universally:
**Workspace → Project → AMT (main/fork) → Blueprint → Steps.**

### What already exists (verified)

- Blueprint steps: `step_index`, `action`, `depends_on`, `pipeline_id` —
  ordered, produced by orchestration + simulation from the AMT (the
  source-of-truth plan). The 100%-match reuse path and branch-coverage
  reconciliation guarantee step/branch alignment.
- Task records: per-step status, `context_assembled`/`context_sources`,
  restart-preservation (source-tagged reconciliation marks interrupted
  tasks), `/task/step/rerun`.
- Loop queues: `amt_candidates.rs` (the unified expansion candidate
  store — 3 routes) and meta_loop's gap list. REAL queues, but
  in-process/JSON-file scoped — not part of the task lifecycle.
- Consciousness review pass: manual-trigger, not queued.

### The design (what this section commits to)

1. **Steps are the only unit of work.** Loop candidates (AMT deepening,
   meta-loop drafts) and consciousness passes STOP being shadow-queue
   entries: each registers as a real task record (source-tagged, per
   directive #26's lifecycle+restart preservation) with its AMT
   container/blueprint markers. The in-memory candidate stores remain
   as the DISCOVERY layer; the task store becomes the EXECUTION layer.
2. **The GlobalOrderIndex — a view, not a copy.** One read model joins:
   task steps by status (queued/running/paused/interrupted/completed) ×
   project × AMT generation (main/fork, live/paused) × blueprint, ordered
   by (project, depends_on, step_index, priority, created). Served by a
   `GET /order/global?workspace=&project=&state=` route. Nothing
   duplicates: the index is derived from the task store + blueprint
   containers + AMT generation containers on each read (invalidate via
   the existing graph ripple).
3. **`paused` becomes a real task state** (today only interrupted/queued/
   running/completed/failed exist) — the operator's "what's live vs
   paused" needs a native state, settable from the UI and from
   coordination events.
4. **External work joins through the same door:** shared-context handoffs
   and gamedev bridge operations flow through `/mcp/call` (§3's ordered
   review) → AMT expansion candidates (§5) → registered as steps/tasks
   on the global queue. No side queues anywhere.
5. **The §3 insight envelope reports the global order** — "what's live,
   what's paused, what's next" comes from the index, per actor scope.

### Build order (extends §8)

```
Phase 1b: paused task state + source-tagged registration of loop
          candidates and consciousness passes as real tasks
Phase 2b: GET /order/global (the derived view) + ripple invalidation
Phase 4b: UI — global order view (Batch J1 family) + per-project
          "live/paused" board; insight envelope gains the order block
```

### Relationship to simulation

Per the operator: blueprint steps are conducted FROM the orchestration
AND the simulation — stage 7's predictions and the zero-shot simulation
validate the steps BEFORE they execute; the global order therefore
carries each step's simulation state (predicted/passed/flagged) alongside
its execution state, so the queue shows not just what's next but what's
CONFIRMED next.

---

## §10. Measured model speeds (2026-09-27, post-fix probes — real numbers)

| Provider | Measured | Notes |
|---|---|---|
| **BitNet (i2_s, local)** | **1.54 tok/s end-to-end** (99 tokens / 64.1s wall, short probe incl. model load) | pure-decode rate from earlier sessions' `parse_bitnet_metrics` was ~7-9 tok/s — both honest, different measures: model LOAD (~25-40s) dominates short single-shot calls; decode itself is the 7-9. |
| **OpenRouter** | **DOWN — 401 "User not found" persists after the $10 purchase** | this error means the API KEY itself doesn't resolve to a user (revoked/deleted/wrong account) — the purchase can't help until the key in `config.toml` is regenerated. Operator action. |

**Why orchestrates got faster after the fixes — the honest answer:** BitNet's
decode rate is a hardware constant; nothing about it changed. The speedup
came from eliminating WASTED work: the confetti gate now accepts BitNet's
first valid candidate instead of burning extra 60s+ generations on
retries, empty-response retries stopped cycling, and the degraded-Ok
extraction path is now visible (capture) instead of silently redoing
work downstream. Fewer calls × same per-call speed = faster end-to-end.

**The real throughput levers (measured-number-driven, per methodology 38):**
1. **Keep-warm BitNet** — the per-call `llama-cli` spawn pays ~25-40s
   model load EVERY call. A persistent `llama-server` held by the prompt
   pipeline (serve mode exists: `ozone_serve.rs` pattern) would cut
   short-call wall time roughly in half-or-better. Architecture
   candidate, not yet built.
2. **A working OpenRouter key** ($10 plan = 1000 req/day) — primary
   models answer in 2-20s; BitNet drops to true fallback. This is the
   single biggest throughput unlock and it is BLOCKED on the 401.
3. **Nested-JSON fix (landed)** — successful multi-step responses
   (blueprint/simulation) now pass the gate on attempt 1 instead of
   being retried into the fallback chain.

---

## §11 (queued full write) — see CHECKLIST 2026-09-27/28 entries for the captured threads: MCP-as-AMT-expansion, tool/MCP call sites in stages 4+7, edge-identification K-registry family, image+3D.

## §12. SECURITY MODEL — how agents interact with Ozone (2026-09-28, operator directive: capture security, prevent terminal-MCP abuse)

**The layered posture — every layer enforced at a different point:**

| Layer | Enforced by | Covers |
|---|---|---|
| Agent identity | Ed25519 device auth (`/auth/*`), QR pairing | who can hold a session at all |
| Metering | `/mcp/call` usage ledger, `OZONE_MCP_DAILY_LIMIT` per agent/day/tool | runaway or abusive call volume |
| Jurisdiction gate | runs on tool input like any input, host-side | content-level policy, always first |
| Tool-side hardening | the MCP's OWN checks BEFORE execution | what the tool will actually do |
| Order review | the Phase-1 insight envelope | visibility: every call leaves captured/missed/order state |
| Graph provenance | ripple + capture stores (S10/S11/S12) | after-the-fact audit of everything |

**Terminal MCP hardening (all live, verified by test):**
1. **Allowlist REQUIRED** — `OZONE_TERMINAL_ALLOW` unset = every exec refused ("locked"). The MCP cannot boot wide-open.
2. **First-token prefix match** against the allowlist; `rm -rf /` refused live (not in list).
3. **Shell operators rejected** (`; | & \` $ < >`) — no chaining smuggled past the allowlist; `git status | head` refused live.
4. **Timeout cap** 600s hard, **output cap** 100KB, cwd must exist.
5. **Loopback-only** binding (`127.0.0.1:3215`).
6. **Optional shared secret** — `OZONE_TERMINAL_TOKEN` set → `/call` requires `x-ozone-terminal-token`; defends against another LOCAL process bypassing Ozone's metering by hitting :3215 directly. (The honest gap this closes: loopback + allowlist alone still lets any local process call the port directly — the token makes direct-calls third-layer-defensible. RECOMMENDED ON.)

**Honest gaps (captured, not hidden):**
- `/orchestrate` still has no session-token validation (CC's long-standing finding) — localhost-only posture today, must close before any exposure.
- `/mcp/call` itself authenticates the TOOL, not the CALLER — caller identity is self-declared (`agent` field). Real enforcement arrives with per-session tokens on this route (design: reuse Ed25519 session token as caller identity; metering keys off the validated identity).
- Bridge `/execute` endpoints (ports 3210/3215) are loopback-bound but unauthenticated unless a token is configured — same recommendation as terminal: set tokens when multi-user.

**Multi-agent review (design, operator-noted):** the review slot in `/mcp/call`'s envelope is currently fact-assembly from real stores. The upgrade path: high-stakes calls (dangerous tool classes, jurisdiction RequireConfirmation results) route through the REAL decision-review machinery (multi_pass_review) — with the model chain SELECTED and ENABLED per config, and N models voting when configured (the multi-agent review step). Gated by config, consistent with "if enabled and selected if available" — never a hard dependency on model availability.

## §13. VERSIONING + PLATFORM — registry status (honest)

**Wired today (verified live):**
- `/mcp/tools`: registration REPLACES existing entries (`"replaced": true` observed on re-register) and records `server_version` — terminal tools re-registered at terminal-0.2.0 with `platform:linux/x64` capability tags. Version recorded + replaceable.
- `/pipelines/remote`: bridges register with heartbeat refresh; 200-203 live.
- Pipeline registry: 82 entries with SemVer (compile-time + seeded + remote).
- Usage ledger: per agent/day/tool — the call-count dimension of version health.

**Not wired yet (gaps, captured):**
1. **No version-constrained dispatch** — callers can't say "only tool >= 0.2"; registry stores versions but nothing enforces ranges. Design: dispatch-time check when a call carries `min_version`.
2. **No platform applicability field on the generic registry entry** — platform lives in capability TAGS today (`platform:linux/x64`, verified) because the register schema has no native field. Design: add optional `platform` to McpToolRegisterRequest; host refuses dispatch on mismatch.
3. **No update policy** — replacement is last-write-wins; no signature/pinning for registered tools. Relevant when third-party tool servers register (currently all registrations are ours).
**Platform today**: Ozone host + BitNet + terminal allowlists are LINUX-first (this machine: linux/x64). The bridges MCP and terminal MCP are Node = portable, but their ALLOWLISTS/paths/ports are per-instance config; platform is now declared in capability tags so misregistration is visible.

---

## §11. FULL WRITE — MCP-as-AMT-expansion, tool call sites, the edge-identification family, and the next modalities (2026-09-28, operator threads captured 09-27 now formalized)

### 11.1 MCP calls ARE AMT expansions

Every tool call changes what a project's plan-under-standing knows, so
every `/mcp/call` deliberately rides the existing GraphRipple route:
the call emits a scoped GraphEvent → `amt_loop::spawn_graph_ripple_sync`
appends an expansion candidate for the affected project's AMT → the
review loop consumes it identically to every other candidate. Tool calls
tag their modality scope (which pipelines the tool's results feed), so
expansion lands on the right AMT. Zero new machinery — deliberate wiring
of what exists.

### 11.2 Tool/MCP call sites in blueprint + simulation

Same doctrine as pipeline call sites: what stages 4 and 7 can SELECT is
what the system can DO. Today steps coerce to pipeline 9 (+ web-search
carve-out). The upgrade, additive:
1. `BlueprintStep` gains `capability_ids: Vec<u64>` alongside
   `pipeline_id` — capabilities are registered tools (from /mcp/tools)
   or modality pipelines.
2. Stage 4's prompt lists available capabilities (name + description +
   platform tags) so the model can select them; the mechanical coercion
   stays for anything unmatched.
3. Stage 7 (simulation) simulates capability calls TOO — including
   creation of new pipelines, tools, and MCPs as simulated steps
   (operator: "in simulation we simulate creation of pipelines tools and
   mcp's as well"). Simulated capability calls carry predictions the way
   model steps do.
4. Every capability call is a registered call site: S11-family capture +
   the /mcp/call usage ledger (the parallel surface for non-model
   capabilities).

### 11.3 The edge-identification K-registry family

The F.1–F.6 edge vocabularies are mostly zero-shot-identified: semantic
edges (Contradicts, CausedBy, LogicallyImplies, LeadsEyeTo,
TemporalPrecedes, Occludes...) are model judgments; structural edges
(Contains, FollowsStep, spatial-from-detection) stay mechanical.
**New K-registry kind: `edge-identification`.** Each modality registers
a strategy preset:
```
EdgeIdentificationStrategy {
  modality: "text" | "code" | "math" | "image" | ...,
  prompt_template,          // offers that modality's edge vocabulary
  output_schema,            // typed edge shapes (from the F.x specs)
  batching: "per_document" | "per_proof" | "per_scene" | "pairwise_window",
  deterministic_edges: [...],  // built without model calls
}
```
- **When**: at creation/analyze in-pipeline (first pass) + ripple-driven
  incremental re-review on graph writes.
- **How it scales**: batched calls (one call returns N edges — the
  E2/E7 shape), per-modality strategies fan out IN PARALLEL (independent
  registries), merged through the single graph choke point.
- **First consumer**: text's disconnected grammar path — the
  cross-sentence-relationships + coreference payload (line-5739 call)
  already extracts the data; this family types it as real TextEdgeType
  edges and wires it to CreateGraph.

### 11.4 Image (102) and 3D (109) go live

Same revival path math took: CLI contract fix (`--input` + envelope
unwrap) → real ZSEI persistence (containers + object_store_path) → graph
retrieval → link_related_containers. Image's target schema is F.3
(SpatialAbove/Below/Left/Right, Occludes, FocalSubject, LeadsEyeTo,
SpatialRelationship, Affordance) — and the **visual MCP** (NEW_MCP_GUIDE
§4) is its natural first consumer: screenshots in, spatial graphs out,
UI automation tools out. 3D (109) gets the same path with scene-graph
types.

### 11.5 Registry persistence (LANED 2026-09-28, binary 21:48)

`/mcp/tools` registrations now survive restarts: McpRegistry persists on
every register/unregister to `{data_dir}/mcp_tool_registry.json` and
restores at boot (boot log line when >0 restored). Closes the gap found
live when the 22:41 restart wiped the 69 registrations.
