# Acting-Factors Registry — everything that can act in Ozone-Studio, and how (2026-10-29, cc)

> Operator ask: don't wire the browser/mobile MCPs "into the Personal Assistant". The assistant works *alongside* everything
> else; MCPs are a shared surface; consciousness acts by **reviewing/monitoring**, not by calling tools. Look at all guides,
> registries, ripple, assistant, I-Loop, file beacon, MCPs, shared context, and say what the **acting factors** really are.
> Sources read for this registry: `ACTING_LOOP_GUIDE.md`, `PERSONAL_ASSISTANT_GUIDE.md`, `UNIVERSAL_ORDER_GUIDE.md`,
> `guides/file-beacon-design.md`, `guides/CONSCIOUSNESS_GUIDE.md` (headings), `TOP_DOWN_REVIEW_GUIDE.md`,
> `REVIEW_WATCHDOG_2026-10-07.md`, `MCP_TESTING_AND_COORDINATION_GUIDE.md`, and the code (`src/lib.rs` boot block,
> `src/orchestrator/actors.rs`, `amt_candidates.rs`, `src/mcp_supervisor.rs`). Evidence tags: **[code]** read in source,
> **[live]** observed running, **[doc]** stated in a guide only, **[unverified]**.

## 0. Correction of my own earlier framing

I wrote (MASTER_REMAINING_WORK_GUIDE §2/W2) that browser actions should be "proposed by the Personal Assistant → approved →
executed via `/mcp/call`". That conflated three different things and is retired:

1. The **Personal Assistant never calls tools.** It is "the consciousness's operator-facing voice" [doc PA §1]: it derives a
   feed from the Order + ripples + beacon history and, for real follow-ups, **creates tasks** through the same enqueue path the
   review pass uses [doc PA §4.2]. "Never reorders the plan… proposals only" [doc PA §4.3].
2. **Execution belongs to the approval-gated task executor and the orchestrator** (`OZONE_AUTO_EXECUTE_TASKS=1`, one queued
   *prompt* task per pass through the full gated walk; non-prompt tasks stay records) [code lib.rs ~1257].
3. **MCPs are a shared tool surface, not a PA feature.** Any agent (operator, zcode, CC, a pipeline once `capability_ids`
   exists) calls them through the one door `/mcp/call`; the call itself is a **ripple source**.

## 1. The doctrine in one picture [doc ACTING_LOOP §3, code]

```
ANY write (ZSEI Create/Update/Delete/Link) ─┐   the universal choke point  src/zsei/mod.rs:191-196
MCP call  → context_mirror(kind:tool_call) ─┤
coordination note/claim/handoff/finding ────┤──▶ GraphEventHub (ripple)
file beacon (45 s poll) → UpdateContainer ──┤
task lifecycle (emit_task_ripple, 3 sites) ─┤
config_change (real saves) ─────────────────┘
        │
        ▼  FREE, local, sync (never a model call)
ActorRegistry: RippleActor.interested_in(evt) → one local fetch → evaluate()  ── an actor is a GATE, not a reviewer
        │ match
        ▼
amt_candidates::append (free, deduped, capped) ─▶ AMT_WAKE ─▶ amt_loop: ONE paid LLM call per candidate (deepen a branch)

Observers (windows-first, read the same ripple/monitor surface):
  consciousness (I-Loop, review pass, gates, experience/emotion) ─▶ reflections, review tasks, decisions, records
  Personal Assistant (check-up loop)                              ─▶ /assistant/feed (derived) + real follow-up tasks
Order (/order/global) = derived view of tasks; the executor (opt-in) runs queued *prompt* tasks through the full gated walk.
```
"The acting loop is the living network." Cross-MCP coordination is **the graph** — no MCP calls another MCP [doc MCP_TESTING §4].

## 2. Registry of acting factors

### 2a. Senses / ripple sources (they create events; they do not decide)
| Factor | Where | Cadence / trigger | Cost | Switch | Evidence |
|---|---|---|---|---|---|
| ZSEI write choke point | `zsei/mod.rs:191` | every write | free | always | [code] |
| File beacon | `file_beacon.rs`, spawned in lib.rs | 45 s poll of registered FileReference containers; JSONL history + snapshots | free | always | [code][live per PA guide] |
| Task ripples | `task/mod.rs emit_task_ripple` | create / status change | free | always | [code] |
| MCP tool-call mirror | `grpc/mod.rs ~3159` → `context_mirror` | every `/mcp/call` | free (+ metering ledger) | always | [live: browser/android calls appear as host events] |
| Coordination mirror | `context_mirror.rs` (note/claim/finding/handoff) | `note_add`/`file_claim`… | free | always | [live] |
| Config-change mirror | `grpc/mod.rs ~1646` | real saves | free | always | [doc] |
| **Sensor tick** | `actors.rs spawn_sensor_tick` | every 900 s, rotates firewall_status → listening_ports → process_top via `call_global`, mirrors as a tool_call event | free + metered | always (no-op if tool unregistered) | [code][live: EU 133 events, all success] |
| Peer-sync review | `network/mod.rs spawn_peer_sync_review` | received envelopes from PAIRED peers | free | `network.enable_p2p` | [code] |

### 2b. Actors (free gates on the ripple) [code actors.rs]
| Actor | interested_in | queues AMT-candidate route | Notes |
|---|---|---|---|
| `SecurityFindingActor` | CoordinationEvent, kind tool_call | `SecurityFinding` | watches only the four original security tools failing; **does not see browser/android/terminal failures** |
| `CoordinationFindingActor` | CoordinationEvent, kind finding | `CoordinationReceived` | shared-context "finding" notes |
| `TaskFailedActor` | Task container, status failed | `TaskFailure` | |
| `CodeStalenessActor` | FileReference, event updated | `CodeStaleness` | only real change events, not beacon baselines |
Project-scoped routes that do not need actors: `UnverifiedNode`, `ThinTree`, `GraphRipple` (`amt_candidates.rs`).
Finding routes dedupe on (container, route, detail digest) [code]. Budget discipline: filters are the lever, never a silent cap
(~20–30 candidates/day target) [doc ACTING_LOOP §3.4].

### 2c. Loops and daemons (boot-spawned in `src/lib.rs`) [code]
| Loop | Cadence | Paid? | Gate / switch |
|---|---|---|---|
| AMT re-expansion (`amt_loop`) | event-woken (`AMT_WAKE`) + 1800 s | **1 LLM call / candidate** | always |
| Graph-ripple sync → candidates | event | free | always |
| Ripple-actor dispatch | event | free | always |
| Methodology meta-loop | 1800 s | LLM | always |
| I-Loop (self-reflection) | 1800 s floor (60 s doc value is budget-incompatible) | LLM | `[consciousness] enabled` |
| **Personal Assistant check-up** (`consciousness/assistant.rs`) | 1800 s floor + task-ripple wake; change-detection skips unchanged findings ("paid check-up skipped") | free filter, then **1 batched call** | `[consciousness] enabled` [live: "findings unchanged… skipped"] |
| Integrity monitor | continuous | free | always (read-guard fix, see lib.rs deadlock note) |
| OpenRouter quota monitor / catalog refresh / free-model probe | periodic | free | key present |
| MCP autostart supervisor | boot + on demand | free | per-tool autostart; knows ports in `facts_for_port` |
| **Task executor** | 60 s tick, one queued *prompt* task / pass | full gated walk | **opt-in `OZONE_AUTO_EXECUTE_TASKS=1`**; memory gate `OZONE_MIN_AVAIL_MB` (default 1536) after two silent host deaths |
| P2P auto-broadcast bridge + batch driver | event / `batch_sync_interval_secs` | free | `network.auto_broadcast_sync` (default false) |
| Review pass (consciousness) | on demand route `/consciousness/review_pass` | creates tasks | S10 + S11 inputs; S12/S13 gap per PA §5 [doc, **unverified now**] |

### 2d. Gates (act by stopping, warning or confirming)
| Gate | When | Mechanism |
|---|---|---|
| Jurisdiction gate | orchestrator stage 0 (Block early) and stage 14 | rules by action: Block stops, Warn surfaces, RequireConfirmation → `decision_review` (pipeline 39 intercepted, graph-context prompt, retry-on-confetti, fail-closed `ReviewPending`) |
| Consciousness gate | stage 8 (full plan + simulation) and `pre_task_gate` at task enqueue (real keyword gate, threshold 0.7) | decision_review + GateDecisionRecord |
| Session gate | `OZONE_REQUIRE_SESSION=1` middleware on PROTECTED routes; `x-oz-internal` for local tools | 401 without session/token |
| MCP usage gate | `/mcp/call` ledger per agent/day/tool | refusal is itself a metered event |
| Terminal/android/browser policies | inside each MCP (allowlists, origin guard, locked-by-default) | refusal as 200 + `success:false` |

### 2e. Actors that are people/agents
operator (UI/CLI/Electron), zcode, Claude Code (and forks), the shared-context MCP (presence, claims, handoffs), device
sessions (paired phone/browser). They act by calling routes and `/mcp/call`; none has a private bypass [doc].

## 3. How consciousness "acts"
By reviewing and monitoring, never by calling tools [doc ACTING_LOOP; PA §1]. Its outputs are: experience/emotion/reflection
records, gate decisions, **review tasks** (aged review-failed S10 rows, S11 zero-shot failures), I-Loop reflections persisted
to the graph, and (through the assistant) a derived feed. Its inputs are the same monitor surface as everyone else: the
ripple, the capture quartet S10–S13 (`/capture/*`), the Order, and graph traversal. "Is this how consciousness acts?" — Yes:
**observe → record → (candidate | task | feed | gate decision)**. Anything that executes a tool does so through a task or an
orchestrated step, under the gates in §2d.
Verified side-effect: on 2026-10-29 the I-Loop reflected on the local firewall failure echo [live host.log]; that echo was a
tool-contract defect (methodology 45), not consciousness misbehaving.

## 4. How the Personal Assistant stores and obtains its list
- **Stores nothing new.** The feed is *derived on read* from `/order/global` (time buckets), last-ripple timestamps, beacon
  history and capture rows; entries carry their source refs [doc PA §4.2, UNIVERSAL_ORDER §1]. "Derived views, never copies."
- **Obtains** its triggers with a free local filter: overdue/due-soon, stalled progress, paused-too-long, meeting-soon,
  slippage cascade, check-up due, stuck step [doc PA §4.1]. Only classes that fire are batched into **one** LLM call; nothing
  firing = silent and free. Unchanged findings skip the paid call entirely [live].
- **Acts** only by enqueuing real follow-up tasks (the Order stays the single store) and by speaking in `/assistant/feed`.
- Same switch as the I-Loop (`[consciousness] enabled`), 1800 s floor, never fabricates urgency or percentages.

## 5. Where the browser / mobile / security / terminal MCPs sit
| Question | Answer |
|---|---|
| Who may call them? | any agent via `/mcp/call`; later pipelines via `capability_ids` (stage 4 listing / stage 7 simulation — **not built**) [doc TOOLS_PIPELINES §11-13]; executor steps |
| Do their calls act on the system? | they ripple: each call is a CoordinationEvent (tool_call) visible to actors, consciousness, the monitor, S13 |
| Does anything react to them today? | only `SecurityFindingActor` for the 4 original security tools. **browser_/farm_/android_/terminal_ failures are not watched by any actor** |
| Should the assistant drive them? | no. If an action should happen, it is a task; execution is the gated executor/orchestrator. The assistant may remind/propose |
| What would "react" mean? | a generic **McpFailureActor** (interested_in tool_call, failure on a registered non-security tool, dedupe per tool) — *where applicable, not forced*; and `exposure_report.external_risky > 0` as a security signal (needs the content-pointer read) |
| Gaps to close first | S13 (tool calls) into the consciousness review pass [doc PA §5]; `capability_ids`; supervisor `facts_for_port` entries for browser (3280) / farm (3281) / code-graph (3279) |

## 6. Switch and environment inventory (acting-relevant)
`[consciousness] enabled` (I-Loop + assistant) · `OZONE_AUTO_EXECUTE_TASKS=1` (executor) · `OZONE_MIN_AVAIL_MB` · `OZONE_REQUIRE_SESSION=1`
· `network.enable_p2p` / `auto_broadcast_sync` / `batch_sync_interval_secs` · `OZONE_JURISDICTION_FORCE_SYNC=1` · per-MCP:
`OZONE_TERMINAL_ROLES(_FILE)`/`_ALLOW`/`_TOKEN`, `OZONE_ANDROID_SHELL_ALLOW`/`_ALLOW_EMULATOR`/`_TOKEN`,
`OZONE_BROWSER_ALLOW_HOSTS`/`_ALLOW_EVAL`/`_NO_SANDBOX`/`_ALLOW_GLOBALS`, `OZONE_FARM_URL`/`_API_KEY_FILE`/`_ALLOW_HOSTS`.

## 7. Findings from this review (new, with evidence)
1. **Port collision I introduced** — browser-mcp `:3250` = threed-mcp in `facts_for_port`; farm `:3252`. Moved to `:3280` / `:3281`
   (code, READMEs, registrar, launcher, selftests, guide); persisted registry endpoints patched (24 entries; backup kept). The
   host must be re-registered when next up. [code][live]
2. `docs/TOOLS_REGISTRY.md` lists connectome `:3250` / threed `:3255`, contradicting the supervisor map (threed 3250, connectome 3255) — doc defect. [code]
3. Local `firewall_status` echo (fixed, methodology 45); verify at the next sensor tick. [live]
4. EU sensor tick healthy; zcode's "0 EU sensor events" was a log-reading artifact. [live]
5. No actor watches non-security MCP failures (see §5). [code]
6. The host was found stopped ("Shutdown signal received" 10:15:26) while another agent has uncommitted edits in
   `src/consciousness/assistant.rs` / `mod.rs` — not restarted by me to avoid colliding with a rebuild. [live]
7. Methodologies 45–50 captured (`assets/methodologies/`, indexed in `src/bootstrap.rs`) — **index edit is uncompiled**.
