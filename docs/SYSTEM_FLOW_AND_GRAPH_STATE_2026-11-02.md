# System flow, gating, action and graph state — top-down capture (2026-11-02)

Scope: one document that follows a request from the entry point to the last ripple, names where every guardrail sits, says honestly how an *action* is produced and ensured, and states the graph state. Everything below was read from the tree (and the local data dir for the measured numbers); **nothing in the 2026-11-01/02 edit wave has been compiled or run** — the build/test pass is operator-gated. Sibling registries: `CALL_ROUTE_REGISTRY.md` (routes × guardrails), `ZERO_SHOT_CALL_REGISTRY.md` (prompts), `CONTEXT_REGISTRY.md` (context carriers), `GRAPH_RELATIONSHIP_REGISTRY.md` (edges), `TOOLS_REGISTRY.md` (MCP fleet), `ACTING_FACTORS_REGISTRY.md`, `REVIEW_E2E_STATE_2026-11-01.md` (the review + edit TODO).

## 1. The whole shape in one picture

```
 client (UI chat: global/meta | workspace | project) ──► /orchestrate  (G1 session gate, G2 entry gate/trace root)
        │                                                    │
        │   timeouts/recovery: submit_only + /orchestrate/status,  task id recovery, awaiting-aware polling
        ▼                                                    ▼
  ┌──────────────────────── orchestrate_traced ────────────────────────┐
  │ state: request, model window, capability summary (MCP registry),   │
  │ pipeline index, trace id                                           │
  │  0  jurisdiction (soft)   1 input   2 normalize+chunk+graph files │
  │  3  gather methodologies + categories + applicable tools/MCP       │
  │  4a classify file graphs  4b modality graphs (+ cross-modal until stable)
  │  5  build AMT (graph-native or chunk zero-shot) ─► clarification? stop
  │  6  blueprint assignment   7 zero-shot simulation                  │
  │  8  consciousness gate (META-ONLY; cannot judge ⇒ AWAIT)           │
  │  8b/14 jurisdiction FULL gate on the resolved plan                 │
  │  9  task creation   10 step execution (+ on_step_complete hook)    │
  │  11 result collection  12 post-execution consciousness             │
  │  13 delivery: response graph → render ladder → link_walk_content   │
  └─────────────────────────────────────────────────────────────────────┘
        │ every write ─► ZSEI::query choke point ─► graph_events::emit (the living-graph ripple)
        ▼
  chat_store: ChatTurn (full prompt + full response) + response graph  ──► /chat/turns
        ▼
  ripple subscribers: UI websocket · ripple ACTORS · AMT ripple sync · P2P (opt-in) · assistant/I-Loop wake
```

## 2. Stage by stage (what it does, what it writes, which guard sits on it)

| Stage | What | Writes / emits | Guardrail |
|---|---|---|---|
| entry | memory-floor gate for walk launch; I-Loop wait; voice input transcription (headless) | trace root, `orchestration_events` | G1, G2 |
| 0 | jurisdiction **soft** pass on the raw prompt (always on, independent of consciousness) | gate result on state | G3; RequireConfirmation → human modal ONLY here |
| 1 | input capture | stage record | — |
| 2 | attached files → per-modality **file graphs** (real content read), prompt chunking, keyword/topic/entity extraction, prompt **chunk graphs** | file/chunk graph containers | G7/G8 for model calls |
| 3 | methodologies by keyword search (floor: root children, else create the Base Orchestration Methodology — creation point 2 of 2); categories (auto-create); **tools/MCP aggregation — PROVIDER BINDINGS FIRST (updated 2026-11-04)**: the gathered methodologies' per-rule `requires[]` (providers = MCP servers) are evaluated against the registry into `state.tool_manifest` (`registered|degraded|missing`); missing required ones are captured (`capability_gaps.json` + `CapabilityGap` meta ripple + methodology gap); `applicable_tools` = bound providers' tools, then request-keyword matches as the FALLBACK (`source:"search"`); **the old "first 8 registry entries" floor was REMOVED — we either have a capability or we don't** | `state.methodologies`, `state.categories`, `state.tool_manifest`, `state.missing_capabilities`, `state.applicable_tools` | consciousness guardrail on the tool list |
| 4a | classify attached file graphs (primary / supplementary / raw_data) | classification on state | `metered_execute_resilient` |
| 4b | aggregate root modalities → create one **modality graph** per verified modality → **cross-modal reference passes until 5 consecutive stable passes** | modality graph containers, `RelatedTo` edges, CrossModalIndex | pipeline handlers; 15 modality `LinkToModality` handlers (edit wave) |
| 5 | **AMT** (Abstract Mind Tree): graph-native traversal when chunks carry sentence nodes; else layered zero-shot (intents → branches → details → cross-refs, validated by a 5-streak) + methodology cross-referencing/synthesis; persisted as an AMT container; unverified nodes enqueue **re-expansion candidates** | AMT container, candidates file | `metered_execute_resilient`; clarification stops the walk |
| 6 | **blueprint assignment**: reuse a ≥95 % matching blueprint (steps from its content file; empty-step blueprints are NOT reused) or design a new one from methodologies + AMT + pipeline catalog + capability summary; each step = `{pipeline_id, action, dependencies, optional model override}` | Blueprint container + `blueprints/walk_*.json` body | G7/G8; paid/unknown model overrides refused (G9) |
| 7 | zero-shot **simulation** of the blueprint with AMT traversal (feasibility, clarification; provider-step availability is host-computed — see §6b 2026-11-04 addendum; the model is no longer the only source of tool selection) | `simulation_result` | fallback-wired; known truncation history fixed by the no-cap policy |
| 8 | **consciousness gate** (pipeline 39 intercepted by `DecisionReviewExecutor`: multi-pass chunked review, per-chunk judged cache, safety-first merge). **Doctrine: only meta/consciousness decides.** Unjudgeable (rate limit, empty, confetti, all failed) ⇒ durable **awaiting item** (`awaiting_items.jsonl`), re-judge only unjudged chunks with backoff, resume the same walk; never fail open, never a human | S10 decision rows, awaiting rows | G6 |
| 8b/14 | jurisdiction **full** gate on the resolved plan | gate result | G3 |
| 9 | task creation via TaskManager (blueprint, AMT container id, trace id recorded) | Task record, ripple | — |
| 10 | **step execution**: dependency order; per step build prompt (original request whole + step + compacted context), optional tool context, run the step's pipeline/model with fallback walk and lane splitting; **`on_step_complete`** reviews AMT alignment when a step reports `graphs_updated` and may synthesize new steps | step results, `graphs_updated` | G3 per-step gate (installed at boot) on tool calls; G5/G10 via `call_global` gate |
| 11 | result collection: complete/fail the task (never overrides a user cancel) | task output | — |
| 12 | post-execution consciousness: experience / core-memory / relationship / emotion pipelines (41/47/43), real per-pipeline outcome recorded | consciousness stores | G6; dispatch gap open (§8) |
| 13 | delivery: voice identity, **Response Graph** assembly + render ladder (content leaves fixed; parse-back audited), dashboard + recommendations, `link_walk_content` (RelatedTo between containers this walk created, idempotent, conservative 2-shared-term floor) | response, edges | — |
| after | **`chat_store::persist_chat_turn`**: ChatSession/ChatTurn (full text) + background response graph | chat containers | §4 |

## 3. Gathering: methodologies, blueprints, pipelines, tools/MCP

- **Methodologies** (ZSEI `Methodology` containers, 45 shipped + 14 created locally): found by keyword (stage 3), also written by two creation points — the orchestrator at runtime (never zero) and the **methodology meta-loop** (drafts from detected real gaps, `{"skip":true}` allowed).
- **Blueprints** (30 locally): stage 6 reuses ≥95 % matches, else designs; body in a content file (`blueprints/walk_*.json`), container carries metadata + keywords + methodology ids.
- **Pipelines**: the catalog (`available_pipelines`, index.json) is part of blueprint design; each step names a pipeline id. Modality pipelines (27 + UiDocument 127) are separate binaries run by the registry executor under the admission gate.
- **Tools/MCP**: `state.capability_summary` (one line per registered tool) goes to blueprint/simulation; `state.applicable_tools` (stage 3) is the shortlist. **Status (updated 2026-11-04):** a step's TYPE is now `pipeline` (default; pipeline 56 keeps its web-search special case) or `provider` — an MCP PROVIDER step created from a methodology rule's declared `requires[]` (or proposed by the blueprint model, host-validated) that runs BEFORE the step carrying the rule through the gated/metered MCP path; which of the provider's tools runs is chosen at call time from the step text (or a pinned `tool`); the result reaches the carrying step as a previous-step output; containers a provider call creates carry `called_from` provenance keywords (provider, tool, site, task, step, rule). The older search-selected `tool_routing` context block (default OFF, `OZONE_TOOL_ROUTING=1`) remains the FALLBACK; its fixed text/size caps (200-char query, 2,000/4,000-char results, ≤2 tools) were REMOVED 2026-11-04 (no-cap doctrine; the block is fitted to the window by the caller). Gates unchanged: `<<<UNTRUSTED_DATA>>>` wrapping, per-step jurisdiction gate, agent `orchestrator-step`, a verdict row per attempt.
- **New tool surface (edit wave):** `document-mcp` (:3282) — the text modality calls `document_extract` for PDF/Office; extraction is a tool, not a pipeline dependency.

## 4. Graphing — what is graphed, when, and the current state

### 4.1 What becomes a graph (writers)
| Source | Container types | Linked by |
|---|---|---|
| attached files (stage 2) | per-modality graph (`ModalityGraph` + modality-specific nodes) | `link_to_existing:true` at creation |
| prompt text spans (stage 4b) | one graph per verified modality | cross-modal passes (until 5 stable), `[13.link]` post-walk |
| AMT | `Derived` AMT container (+ methodology/blueprint refs, `Continues` root relation across prompts) | structural children, candidates |
| methodologies / blueprints / tasks | `Methodology`, `Blueprint`, `Task` | keywords, ids |
| **response text** (new) | text graph per turn | `ChatTurn —RelatedTo→ graph` |
| **chat** (new) | `ChatSession` (per scope) → `ChatTurn` | structural parent; `RelatedTo` to prompt graphs |
| tool outputs | `McpResult` / `McpEntity` (`mcp_graph.rs`, modality inferred from tool-name prefix) | edges from the tool's graph block |
| UI structure / pixels | UiDocument graph (127) ↔ Image container (`Renders`) | auto-ingest loop |
| coordination / findings | `CoordinationEvent`, finding containers (S13 mirror) | scope keywords `proj:`/`file:` |
| consciousness | experience / core memory / emotional / identity / narratives | consciousness roots |
| file reality | `FileReference` (beacon, 45 s poll) | project parent |
Every write goes through `ZSEI::query` → `graph_events::emit` (the single choke point). Edges live in `context.relationships` (`GRAPH_RELATIONSHIP_REGISTRY.md`).

### 4.2 Measured state — local instance data dir (`target/release/zsei_data`, last written 2026-11-01 19:01; host down, nothing running)
| Item | Measured |
|---|---|
| modality graph files in `graphs/` | **258** — text 174, code 41, image 14, then 1–2 each across **24 other modalities** (network, math, imu, hyperspectral, dna, bci, video, thermal, sound, sonar, radar, proteomics, kinematics, haptic, geospatial, electromagnetic, eeg, control, chemistry, cad, biology, audio, 3d_graph) |
| `local/` flat JSON store | 4,319 files (18 MB) — includes the legacy flat Workspace-tab store; not all are ZSEI containers |
| `global.mmap` | 16 MiB (the ZSEI container index — not parsed here) |
| AMT trees (`amt/`) | 131 files |
| methodologies / created | 45 / 14 |
| blueprints | 30 |
| consciousness store | 1,249 files |
| shared-context (coordination mirror) | 1,840 files |
| `chunk_graphs/`, `file_graphs/`, `cross_modal/` | **empty directories** — these graph kinds are held in the ZSEI index/`graphs/`, not these dirs (stale directory names; documented here so nobody reads emptiness as "nothing graphed") |
| AMT re-expansion candidates | 491 rows: **317 `failed_structural`** (the graph-ripple echo problem — fixed in the edit wave by an echo filter at the producer), 11 `failed_after_max_attempts`, 163 legacy rows without an outcome |
| model ledger | 5,503 rows, **63 unreadable** (concurrent partial writes — writer fixed to one `write_all`, loader salvages) |
| `capture/call_spans.jsonl` / `context_records.jsonl` | 3,525 / 5,250 rows |
| `zero_shot_calls` / `pipeline_zero_shot_calls` / `tool_calls` / `decision_review` | 2,891 / 4,399 / 348 / 261 rows |
EU instance (separate): UiDocument graph root 2493 (google.com, 101 entities / 28 relations), farm screenshot container `Renders` edge — see README status.

### 4.3 Graphing gaps found in this audit (all recorded in `CHECKLIST.md`)
1. **The conversation was never a graph** (browser-only transcripts) — fixed: `chat_store.rs` (edit wave, uncompiled).
2. **Silent container loss**: callers passing flat JSON (auto-created Categories, synthesized Methodologies, voice identity) never created containers (`Container` needs `global_state/local_state`; errors were ignored). Fixed in `ZseiStoreAdapter::create_container` (`normalize_container_json`). After the next build expect Category / Methodology / VoiceIdentity containers to start appearing.
3. Workspace-tab Projects/Workspaces were flat JSON (unified into ZSEI containers earlier; the legacy flat store remains as a fast layer).
4. Response text was not graphed — fixed (background text graph).
5. Cross-modal quality is bounded by each modality's handler: 15 modalities gained `LinkToModality` this wave; `uidocument` (lib crate) has none.
6. Modality-internal LLM helpers: 18 pipelines read the wrong env var so their semantic extraction always failed silently — fixed; their findings are "model-judged", not ground truth.

## 5. How an *action* happens, and how it is ensured

There are four distinct kinds of "acting"; conflating them is the usual source of confusion.

1. **Walk action = pipeline execution.** A blueprint step names `pipeline_id + action`; the registry executor spawns the pipeline binary. Ensured by: stage 8 (meta-only consciousness gate, awaiting on doubt), stage 8b (jurisdiction on the resolved plan), the admission gate (concurrency tiers), the rate limiter, the ledger, `Instrumented` call index, `on_step_complete` AMT alignment, and task state (cancel is never overridden).
2. **Tool action = `call_global` / `/mcp/call`.** Only when (a) a PROVIDER STEP runs (step_type `provider`, 2026-11-04) or a step opted into the search-based tool routing fallback (`OZONE_TOOL_ROUTING=1`, read-only context augmentation), (b) the sensor tick runs its fixed read-only posture tool (15 min), (c) the operator approves an assistant proposal (executed as agent `assistant-proposal (operator-approved)`), or (d) an external agent calls `/mcp/call`. Ensured by: session gate (G1), jurisdiction (G4/G5 — `call_global` and `/pipeline/execute` were bypassing it before this wave), usage ledger (G10), `agent_trust`-aware role gates (terminal-mcp).
3. **Knowledge action = AMT re-expansion.** The living graph ripples every write; the **ripple actors** (`SecurityFinding`, `CoordinationFinding`, `TaskFailed`, `CodeStaleness`, and `McpFailure` — the last written 2026-10-31 and still uncompiled) turn matching events into **findings** and append **re-expansion candidates** (routes: UnverifiedNode, GraphRipple, Continuation, and the finding routes); `amt_loop` wakes (event or interval) and asks the model for concrete sub-detail on one unverified branch, rejects placeholder echoes and confetti, persists the deepened tree in place and emits `AMTExpansion` (source `amt-expansion`, filtered out by the ripple sync so it cannot re-trigger itself). `ChatSession`/`ChatTurn` are also non-deepenable. Escalation across passes uses the `meta_fallback` chain; max-attempt candidates end `failed_after_max_attempts`.
4. **Operator-facing action = the Order.** Tasks (TaskManager: queued / running / paused / interrupted / awaiting / done / cancelled) are created by the walk, by `/task/create` (connectors, assistant follow-ups, UI), and shown by `/order/global` (live, paused, queued, interrupted, awaiting, done, other; due buckets; derived recurring entries). Queued-task execution exists only behind the opt-in `OZONE_AUTO_EXECUTE_TASKS=1` (60 s tick, memory-floor gated): ONE queued **prompt** task per pass runs the full gated walk (every gate a chat request passes); non-prompt queued tasks stay records — there is no un-gated action path. The **personal assistant** loop (consciousness-enabled, ≥1800 s floor, woken by task ripples) watches the Order with a free local filter, sends ONE batched digest only when something fires, and now creates deduped follow-up tasks (urgent overdue, slippage cascade per project, just-ended meeting) and writes a `CONSCIOUSNESS_RESPONSE` record per check-up.

**Gate matrix for an action** (all must hold): identity (G1/G2) → jurisdiction (G3 at stage 0 and 8b, G4 per step/call, G5 for MCP HTTP) → consciousness/meta (G6, awaiting) → admission + rate (G7/G8) → free-only (G9) → metering (G10/G11) → index (G12). `CALL_ROUTE_REGISTRY.md` §1 lists every route against these.

## 6. Consciousness, ripple, I-Loop, assistant — who wakes whom
- **Ripple hub** (`GraphEventHub`): subscribers = UI websocket, ripple-actor dispatch, AMT ripple sync, (opt-in) P2P publisher.
- **Consciousness** is a windows-first observer of the same feed (task manager hooks + stage 8/12 + I-Loop); gates are meta-only; humans are involved ONLY for jurisdiction `RequireConfirmation` (modal → `/consciousness/reviews/resolve`) and for operator-approved proposals.
- **I-Loop** (EVENT-DRIVEN since 2026-11-04 — meta ripple + 60 s startup catch-up + 6 h liveness heartbeat, ≥10 min gap between paid passes; the old ≥1800 s timer floor is gone): self-reflection over the store, now shown every signal DATED with its age and the current time; its model calls are captured (`i_loop_reflection`) and now indexed in `call_spans.jsonl`.
- **Methodology meta-loop** (EVENT-DRIVEN since 2026-11-04: wakes on `MethodologyGap` / `CapabilityGap` meta ripples): drafts methodologies from real gaps. **Assistant loop** (event-driven + next-deadline sleep, ≥5 min gap): §5.4. **Meta ripple dispatcher** (`orchestrator/ripple_class.rs`, `GET /ripple/meta/status`): classifies every ripple event; meta events wake the I-Loop, meta-loop, assistant and AMT loop — docs/RIPPLE_CATEGORIES_AND_META_RIPPLE_2026-11-04.md. **Sensor tick**: 15 min. **File beacon**: 45 s. **Awaiting driver** (new): 15 s tick waking parked walks and applying resume requests.

## 7. Context flow capture (what is recorded, joined by what)
`CONTEXT_REGISTRY.md` §8 is the by-route map. Summary: ledger + ContextRecord (S11) + `zero_shot_calls` per model call; `call_spans.jsonl` per executor call (shape only); S10 decision rows + `decision_review.jsonl`; `awaiting_items.jsonl`; usage ledger + gate-verdict rows for tools; **ChatTurn files (full text)**; all joined by `trace_id` / `span_id`. Previews in logs are previews; the full text is always in a store.

## 8. Open items (honest; full list at the end of `CHECKLIST.md`)
- Whole edit wave is **uncompiled**; one build + test pass is operator-gated (host down; disk 93 %).
- Recent chat turns are not yet injected as walk context (reachable through edges/keywords only).
- Awaiting: `consciousness_gate` (meta-resolved, written by stage 8) and `jurisdiction_confirm` (operator-held, written when a RequireConfirmation review is unresolved; closed by `/consciousness/reviews/resolve`) have writers; `meta_gate` is reserved (no second meta gate exists). A walk is in-process, so a restart ends it: boot now marks `awaiting` tasks `interrupted` and cancels their gate items (resubmit) instead of leaving phantoms.
- Stage 12 now verifies the host store's experience + emotion records (pipelines 41/43 never had binaries); pipeline 47 (relationship) is reported unavailable; consciousness-pipeline registry names do not match the asset folders (CALL_ROUTE_REGISTRY §5). `collective_enabled` and `playback_enabled` have no consumer yet (everything else in `[consciousness]` is wired and reported by `/consciousness/state`).
- Direct loopback calls to MCP ports are refused by `tools/shared/guard.mjs` on strict hosts / with `OZONE_MCP_REQUIRE_INTERNAL=1` (the host sends `x-oz-internal`); off by default elsewhere.
- Operator-gated: build/relaunch, EU deploy, commit/push, key rotation, data mutations (registering `document-mcp`, FileRef backfill, dedupe, worktree deletion).


## 9. The missing-response rule
An empty or missing model response is a failed call, never a silent `{}`/default; the per-site inventory is `CALL_ROUTE_REGISTRY.md` §4.


## 10. Span-out, pause/resume
Per-step context fans out from captured relationships (`span_out.rs`), failures pause the task at the failed step and resume from a checkpoint, and the graph create/expand/use matrix is in `SPAN_OUT_AND_RESUME_2026-11-02.md`.


---
## Addendum 2026-11-04 — what changed after this document was written (read with the sections above)
- **Tools are PROVIDERS bound per RULE in methodologies; the BLUEPRINT is where they become steps** (docs/METHODOLOGY_BLUEPRINT_TOOLS_REVIEW_2026-11-04.md Appendix A/B, docs/MCP_TOOLS_MODALITY_ARCHITECTURE_REVIEW_2026-11-04.md). New `DecisionRule.requires[]`; Stage 3 `tool_manifest`; blueprint prompt "RULE → REQUIRED PROVIDERS"; `reconcile_tool_bindings` inserts provider steps before their carriers and renumbers; missing capabilities are captured, never papered over.
- **Stage 3's "first 8 registry entries" floor is gone.** **Text/size caps in tool routing are gone.**
- **Loops are event-driven** (meta ripple) — `ripple_class.rs`; `GET /ripple/meta/status`.
- **Span-out is per-modality** (`graph_fanout.rs`; `POST /context/span_out`): entries → flow/call sequence → structure; cross-file code calls are captured.
- **Loop prompts carry the CURRENT TIME** and every signal is dated (`run_spans::now_line/when`).
- **New read routes:** `/context/span_out`, `/ripple/meta/status`, `/marketplace/overview` (all session-gated; see CALL_ROUTE_REGISTRY §6).
