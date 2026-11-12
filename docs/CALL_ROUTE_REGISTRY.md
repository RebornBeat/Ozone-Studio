# Call Route Registry — every way a model, pipeline or tool gets called, and which guardrails sit on each

Created 2026-11-02 from a code audit (not a build/test run). Sibling registries: `ZERO_SHOT_CALL_REGISTRY.md` (per-prompt inventory), `TOOLS_REGISTRY.md` (MCP fleet), `CONTEXT_REGISTRY.md` (context carriers), `GRAPH_RELATIONSHIP_REGISTRY.md`, `ACTING_FACTORS_REGISTRY.md`.

Rule of the registry: **a route with no row here is a bypass until proven otherwise.** Add the row when you add the route.

## 0. The guardrail layers (what "enforced" can mean)

| Layer | Where | What it does |
|---|---|---|
| G1 session gate | `grpc::session_gate` middleware, `PROTECTED` list (`/mcp/call`, `/task/`, `/capture/ingest_*`, `/mcp/tools`, `/network/`, …) | With `OZONE_REQUIRE_SESSION=1`: bearer / `x-oz-session` / `x-oz-internal` required |
| G2 entry gate | `orchestrator::entry_gate` (`gated`, `gated_checked`, `require_identity`) | trace root/child + one span row per external dispatch; identity precondition |
| G3 jurisdiction (walk) | stage 0 + stage 8b/14 (full gate) | rules (132 files / 648 rules), Block / Warn / RequireConfirmation (human modal only) |
| G4 jurisdiction (per call) | `jurisdiction::build_step_gate` → `tool_routing::set_step_gate` AND `mcp::set_call_gate` (installed at boot, refreshed every 10 min, `lib.rs`) | same rule snapshot on tool-routing steps, `call_global`, `/pipeline/execute` |
| G5 jurisdiction (MCP HTTP) | `grpc::mcp_call` full gate before `invoke` | Block refuses, RequireConfirmation → review |
| G6 consciousness / meta gate | stage 8, stage 12 | **meta-only**; cannot judge → AWAIT (durable `awaiting_items.jsonl`), never fail-open, never a human |
| G7 admission gate | `pipeline::gate` (priority tiers `Loop`, `Lane`, …, ordered admission) | concurrency, not rate |
| G8 rate limiter | prompt binary, cross-process lock-file bucket (`OZONE_OPENROUTER_RPM`, default 20) | OpenRouter per-minute |
| G9 free-only / paid refusal | model resolution + `zero_shot_simulation` policy checks | no paid provider reachable |
| G10 usage ledger | `mcp::UsageLedger` (per agent/tool/day) | MCP daily limits |
| G11 model ledger | `model_ledger` (single-`write_all` rows) | outcome / latency / cause per model call |
| G12 call index | `orchestrator::instrumented::InstrumentedExecutor` → `capture/call_spans.jsonl` | shape-only row per executor call (trace + span) |
| G13 decision review | `DecisionReviewExecutor` (pipeline 39 interception) | multi-pass chunked review, safety-first merge |

## 1. Routes

| # | Route | Entry | Guardrails now | Capture sink | Status |
|---|---|---|---|---|---|
| R1 | Walk model calls (`metered_execute_resilient/raw`) — blueprint, simulation, compaction, web decompose, compliance, file role, AMT detail/domain/synthesis, response render, alignment | `/orchestrate` → orchestrator | G1(entry)+G2+G3+G6+G7+G8+G11+G12 (executor chain `Instrumented(DecisionReview(Registry))`) | ledger, `zero_shot_calls.jsonl`, S10–S13, ContextRecord | ✅ |
| R2 | Walk direct `executor.execute(9, …)` — lane retry (`amt.rs`), fallback walk (`mod.rs`), sub-steps (`stages.rs`), strength-N votes, graph builders | orchestrator | same executor chain (G12 indexes it) | call_spans + ContextRecord via `capture_loop_model_call` | ✅ |
| R3 | Background loops — AMT re-expansion, methodology meta-loop, I-Loop, assistant digest | `lib.rs` boot spawn, `with_priority(Loop)` | G7+G8+G12 (**fixed 2026-11-02**: loops used the bare registry adapter, so no span rows) | `capture_loop_model_call` + call_spans | ✅ (cross-pass escalation via `meta_fallback`, not an in-call walk) |
| R4 | Pipeline-internal LLM — modality `llm_zero_shot` / `SubprocessExecutor` via the prompt binary | modality pipeline process | G8 (the binary is the limiter), G9; **not** G3/G6 (inside an already-gated run) | per-pipeline `capture::capture_zero_shot_call` | ⚠️ see R4 notes |
| R5 | `/pipeline/execute` (external) | HTTP, session token | session validated, consciousness-category refused, G2, **G4 added 2026-11-02** | span row | ✅ (was missing jurisdiction) |
| R6 | `/mcp/call` (external agents) | HTTP | **G1 (now PROTECTED)**, G5, G10, identity forwarded as `agent_trust` | usage ledger, activity hub | ✅ |
| R7 | `mcp::call_global` (**PROVIDER STEPS**, search-based tool routing, sensor tick, approved assistant proposals) | in-process | G10 + **G4 added 2026-11-02** (these skipped jurisdiction); per-step gate for provider steps/tool routing | usage ledger + gate verdict rows (`provider-step` / `tool-routing`) + **post-call hook 2026-11-04**: a tool's `graph` block is persisted with `called_from` provenance and an S13 capture row is written (parity with R6); provider health ledger; the same hook writes the **S13 `tool_calls.jsonl` capture row for every call (success and failure, `transport: call_global`)** | ✅ (the context-mirror coordination ripple is still R6-only — open) |
| R8 | `/task/step/rerun` | HTTP | G1 (`/task/`), session token, G2 `gated_checked` | span row | ✅ |
| R9 | `/capture/ingest_image`, `/capture/ingest_uidocument` | HTTP | G1, G2 | graph write choke point | ✅ |
| R10 | Text modality → document MCP (`document_extract`) | text pipeline → host `/mcp/call` | G1 (internal token), G5, G10 | usage ledger | ✅ new 2026-11-02 |
| R11 | MCP servers that spawn pipelines or call other services (`visual-mcp` → image binary, farm/browser MCPs) | MCP process | upstream `/mcp/call` already applied G1/G5/G10; the spawned binary is a modality pipeline (R4 rules) | MCP-local + graph writes | ✅ by construction |
| R12 | Direct provider HTTP from a non-prompt pipeline | — | none | — | ❌ none known after the `zero_shot_simulation` fix (below). **Any new `openrouter.ai` URL outside `prompt/main.rs`, `model_windows.rs`, `openrouter_quota.rs`, `model_ledger.rs` is a bypass.** |

### R4 notes (pipeline-internal LLM calls)
- Fixed 2026-11-02: `zero_shot_simulation/main.rs` (pipeline 16) POSTed to OpenRouter directly (no limiter, no empty-response diagnostics, 1500 cap). It now spawns the prompt binary (`OZONE_PROMPT_PIPELINE_PATH`); the free-only policy checks still run first.
- Fixed 2026-11-02: 18 modality pipelines (kinematics, haptic, depth, CAD, geospatial, radar, network, electromagnetic, thermal, biology, sonar, 3D, IMU, BCI, control, sound, hyperspectral, proteomics) read `OZONE_PROMPT_PIPELINE` (default `./pipeline_9`) while the host exports `OZONE_PROMPT_PIPELINE_PATH`; their LLM calls therefore always failed silently (`unwrap_or("{}")`). They now read `OZONE_PROMPT_PIPELINE_PATH` first. Remaining weakness: those calls send `system_context` (the prompt binary reads `system_prompt`) and swallow failures into `"{}"` — a modality-by-modality cleanup, not a bypass.
- These calls are inside a walk that already passed G3/G6; they are NOT individually gated and not in the model ledger unless the prompt binary writes the row. Treat any modality feature that makes semantic claims from such a call as "model-judged", never ground truth.

## 2. Output policy — NO response cap (corrected 2026-11-02)

**A generated response is data.** It is stored, graphed, reasoned over and shown; cutting it corrupts our own record (truncated JSON, truncated sentences, and on reasoning models an empty answer because hidden thinking spent the whole budget). Therefore:

- **The prompt binary sends no `max_tokens` on the chat-completions wire.** The caller's value (150–1000 across ~25 sites, `ZERO_SHOT_CALL_REGISTRY.md` §9) is an **answer-length hint** — the prompt text already asks for brief/structured output — and is not forwarded. The provider bounds output by the remaining context window; runaway generation is bounded by the host watchdog, never by truncating the answer. (An earlier same-day revision raised the cap to a 4096 floor — superseded; a floor is still a cap.)
- Operator escape hatch only: `OZONE_API_FORCE_MAX_TOKENS=1` forwards the caller's value unchanged. The model ledger records a cap only in that mode.
- **Local runtimes (BitNet / llama.cpp)** need a numeric `-n`; the *context window* is a hard physical limit there, so generation is bounded by half the window (the other half holds the prompt) — not by the caller's hint.
- The Anthropic wire requires `max_tokens` (default 4096); it is unreachable under the free-only policy.
- **Input** is what gets chunked/windowed (AMT, lanes, decision_review). Output is not chunked.
- **Previews are not storage.** `zero_shot_calls.jsonl` keeps a 500-char `response_preview` and 200-char `prompt_preview` for log scanning only; the full text lives in the ContextRecord, the ChatTurn (below) and the step result. Nothing downstream reads a preview as data.

### 2.1 Chat is a scoped graph entity (new 2026-11-02)
Before this, the host persisted **no** conversation record — transcripts lived only in browser storage, so the full response was never part of the graph. Now `orchestrator::chat_store` persists every `/orchestrate` turn (success, clarification, failure):

| Scope (from the request) | `ChatSession` parent | Session keyword | Content dir |
|---|---|---|---|
| global / meta chat (no ids) | root | `chatsession:global` | `chat/global/` |
| workspace | workspace container | `chatsession:ws:<id>` | `chat/ws-<id>/` |
| project (workspace kept as tag) | project container | `chatsession:proj:<id>` | `chat/proj-<id>/` |

- One `ChatSession` container per scope; one `ChatTurn` child per turn. The turn's content file holds the FULL prompt, FULL response, task / blueprint / trace ids, model, outcome, and the prompt-graph ids. The file is written **first**, so the text survives even if container or graph creation fails.
- The response is then graphed through the text modality in the background and linked (`RelatedTo`) from the turn, as are this run's prompt graphs; `response_graph_id` is written back into the file.
- `GET /chat/turns?project_id=&workspace_id=` returns the stored turns (full text) and the UI hydrates an empty local chat from it (restart / new device).
- `ChatSession` / `ChatTurn` are in the AMT ripple's non-deepenable set (conversation bookkeeping must not trigger re-expansion).

## 3. Open items (audit residue)
- Fallback walk (`run_one_candidate`) has reasoning *salvage* but not the primary loop's doubling retry.
- (closed 2026-11-02) Modality `llm_zero_shot` helpers now send `system_prompt` and return an error instead of silently substituting `"{}"` when the prompt binary returns nothing.
- `meta_gate` awaiting kind is reserved (no second meta gate); `jurisdiction_confirm` is now written when a confirmation review is unresolved and closed on operator resolve.
- MCP servers still do not re-check jurisdiction themselves (they trust `/mcp/call`), but a direct loopback call is now refused: every server loads `tools/shared/guard.mjs`, which requires the host's `x-oz-internal` header when `OZONE_MCP_REQUIRE_INTERNAL=1` or on strict-session hosts (`OZONE_REQUIRE_SESSION=1`); the host sends the header on every tool call and passes the token to the MCP processes it spawns. Off by default on non-strict hosts so a dev box with an older host binary keeps working.


## 4. The missing-response rule (audit 2026-11-02): an empty or missing response is a FAILED call

**Rule:** a model call that returned no response text, or text that does not parse where parsing is required, is a *failure* and must be recorded and handled as one (retry, fall back, await, skip-with-record, or fail the stage). It must NEVER be converted into `"{}"`, `"[]"`, `""`-that-parses, a default struct, "found:false", "no results", "medium feasibility" or "nothing to add". Legitimate empty answers are explicit (`{"details": []}`, `[]` from the model itself) and are distinguished from blanks.

Helpers: `PromptOrchestrator::response_text(&Value) -> Option<&str>` (None for missing/blank) and `parse_json_object_strict` (Err instead of `{}`).

| Site | Before | Now |
|---|---|---|
| Blueprint assignment (`stages.rs`) | `unwrap_or("{}")` + `parse_json_object` → empty "Generated Blueprint" | blank / unparseable ⇒ stage `Err` (loud) |
| Zero-shot simulation (`stages.rs`) | same → "medium feasibility", no predictions | stage `Err` |
| AMT intent / branch-lane (`amt.rs`) | `unwrap_or("{}")` parsed as valid "no intents" | blank ⇒ parse failure ⇒ recorded event (`intent_parse_failed` / `branch_lane_parse_failed`) |
| AMT detail extraction | `unwrap_or_else(default)` silently | `detail_parse_failed` event + warn |
| AMT cross-reference | unparseable verdict silently skipped (looked like "not related") | `crossref_parse_failed` event |
| Methodology domain id | blank/unparseable ⇒ `unwrap_or_default()` = "no domains" | `domain_id_empty_response` / `domain_id_parse_failed`, pass ends |
| Methodology synthesis | unparseable draft ⇒ methodology with a `null` body | `methodology_synthesis_*` event, no container created |
| AMT alignment review (`mod.rs`) | silently "no expansion needed" | `alignment_review_unparseable` event |
| Strength-N votes | blank parsed from `"{}"` | blank = no vote, warned |
| Web-search decompose | silent fallback to the original query | fallback kept (designed), now warned |
| AMT re-expansion loop (`amt_loop.rs`) | no JSON object ⇒ `{}` ⇒ Ok(false) "nothing to add" | `Err` (failed attempt, escalates next pass) |
| Methodology meta-loop draft | blank ⇒ `{}` ⇒ "draft had no rules" | blank counted as a failure, retried next cycle |
| Text modality: sentence extraction | blank/unparseable/errored ⇒ **Exhausted**; a chunk whose every call failed completed with zero sentences | new `ExtractionOutcome::Failed`; 3 in a row ⇒ chunk INCOMPLETE; an empty result keeps the raw chunk text instead of vanishing |
| Text modality: modality / paragraph detection | unparseable ⇒ `{"found": false}` | warned as a failed call |
| Text modality: sentence split, grammar relationships, sentence / cross-sentence grammar | `unwrap_or("[]"/"{}")` and silent `unwrap_or_default()` | blank ⇒ retried then given up loudly; parse failures logged |
| Text modality: yes/no confirmations | blank parsed from `"{}"` | blank = no vote, logged |
| 18 modality `llm_zero_shot` helpers | `unwrap_or("{}")` | `Err` (earlier today) |
| `zero_shot_simulation` pipeline | direct provider call, `"No response"` | via the prompt binary (earlier today) |
| UI chat | "No response generated" / "Response received" | explicit failure-to-answer notice |
Already correct and left as is: decision review (blank ⇒ unjudged ⇒ awaiting, never approval), lane split (blank part ⇒ `Err`), assistant digest (labelled degradation with the raw findings), step-context compaction (blank ⇒ budgeted raw context, recorded), compliance check (retry then `last_error`).

## 5. Consciousness pipelines: registry ↔ crates (corrected 2026-11-02)
The earlier statement that the names "do not match" was incomplete: `pipeline::registry::binary_alias` already linked 39/41/42/43/47 and is now extended to 40, 44, 45, 46, 50, 51, 53, 54 (only 48/49 have no crate) — the table with the evidence is in `SPAN_OUT_AND_RESUME_2026-11-02.md` §5. None of the consciousness binaries is built yet; the host-native store remains the working implementation (stage 12 verifies it; stage 13 no longer dispatches the dashboard "Update" call, which the crate cannot deserialize).


## 6. Additions 2026-11-04 (entry points added or changed this wave)

| # | Route / entry | Guardrails | What it does |
|---|---|---|---|
| R13 | `POST /context/span_out` | G1 (`/context/` PROTECTED), read-only | runs the one span-out algorithm from given seeds; items carry per-modality `fanout{entries, sequence_head, class_counts}` (`graph_fanout.rs`) |
| R14 | `GET /ripple/meta/status` | G1 (`/ripple/`), read-only | meta-ripple counters + per-loop wakes/passes + policy (min gap / heartbeat) |
| R15 | `GET /marketplace/overview` | G1 (`/marketplace/`), read-only | providers (MCP servers) + their tools/health, paired peers (catalog exchange honestly `not_implemented`), methodologies with per-rule `requires` (declared) and keyword-INFERRED providers, blueprints (provider steps), WANTED capabilities, provider health, binding proposals, policy |
| R16 | **Provider step** (`BlueprintStep.step_type == "provider"`) → `execute_provider_step` → `tool_routing::call_provider_for_step` → `call_global` (R7) | per-step jurisdiction gate, metering, internal token; failure pauses the walk (resumable) | calls a PROVIDER (MCP server) chosen from a methodology rule's `requires[]`; tool chosen at call time from the step text (or pinned); result reaches the carrying step; containers get `called_from` keywords |
| R17 | Meta-ripple dispatcher (`ripple_class::spawn_meta_ripple_dispatcher`) | free, no LLM | classifies every graph event; meta events wake I-Loop, meta-loop, assistant, AMT loop |
| R18 | Capability-gap capture (`tool_binding::record_capability_gaps`) | free | required-but-unregistered providers → `capability_gaps.json` + `CapabilityGap` meta ripple + methodology gap |

**Superseded statements in this file:** R7 above (was "tool routing, sensor tick, approved proposals"); §3 residue about tool routing caps (2,000/4,000 chars, ≤2 tools, 200-char query) — REMOVED 2026-11-04 (no-cap doctrine; the block is fitted to the window by the caller). Stage 3 no longer lists "the first 8 registry entries" — we either have a capability or we don't.


## Update 2026-11-07 (uncompiled) — capability closure + walk metrics
| R19 | **Capability closure** (`stages.rs::close_capability_gaps`, between Stage 6 and 7) → per gap: strength-N necessity vote (`confirm_yes_no_orch` → shared `k_validation` primitive) → `draft_methodology_for_capability` (pipeline 12 `Create`) → `replan` through Stage 6 | 1 vote set + 1 draft call per real need; progress-bounded | gaps → `not_needed` / `drafted` in `capability_gaps.json` |
| R20 | **Walk metrics** (`orchestration_events::append_walk_metric`) | free | one line per stage event → `zsei_data/walk_metrics.jsonl`; read by `tools/walk-scorecard/scorecard.mjs` |
Vote strength for every vote route = `k_validation::configured_strength()` (`OZONE_VALIDATION_STRENGTH` ← `[k_algorithms] validation_preset`).

## Update 2026-11-08 (uncompiled) — keyword program routes
| R21 | **Hit/judgment capture** (`keyword_program::record_search/record_event/record_judgment`) | free | `zsei_data/keyword_hits.jsonl`, `judgment_samples.jsonl`; call sites: search door, span-out (`packed`), Stage 3 (`gathered`, E13), stages (E14, closure, `applied`), grpc decision_review (human) |
| R22 | **Rollup** (`keyword_program::rollup_once`, meta loop) | free | log → `keyword_meta` counts + hints, one write per container |
| R23 | **Keyword settlement** (`keyword_settle::settle_once`, meta loop after the gap pass) | 1 call per unsettled container + votes only for proposed removals | verbatim capture → expansion → unanimous-drop judgment |
| R24 | **Model training** (`keyword_program::train_if_data_grew` → pipeline 17 `TrainModel`) | free (pure Rust, offline CPU) | dataset → train → held-out sign test → registry; models are graph citizens |
| R25 | **Ranked search** (`StoreAccess::search_ranked` → `SearchContainersByKeywords{strategy:"token"}` → `keyword_program::order_for_search`) | free | span-out, context_aggregation, text/code/math linking; interchangeability knobs `OZONE_KEYWORD_BACKENDS`, `OZONE_SEARCH_DEFAULT` |

## Update 2026-11-08 (later) — origin propagation and creation capture
| R26 | **Origin propagation** (`keyword_program::Origin`; `orchestrate()` installs it with the trace, `traced()` carries it across spawns, loops install `Origin::actor`, `executor.rs` stamps `OZONE_ORIGIN`, 33+1 pipelines send `x-ozone-trace` / `x-ozone-origin`, `grpc::query_zsei` re-installs both) | free | every `created`, `search` and outcome event carries call_site / workspace / project / global / request terms / file ids; a pipeline-originated search joins the walk's trace |
| R27 | **Creation + lifecycle capture** (`zsei/query.rs::create_container` → `record_created`; `update_container` → `record_keywords_changed`) | free | the ONE write door logs `created` (type, name, keywords, topics, typed scope, parent, provenance, origin) and `keywords_changed` (added / removed) for every writer; dataset builder turns a methodology/blueprint `created` event with request terms into a CREATION PAIR |

## Update 2026-11-10 — keyword validation gate, rule gate, refinement record
| R28 | **Write-time keyword gate** (`zsei/query.rs::create_container` / `update_container` → `keyword_program::gate_keywords_on_write`) | free | shape rules (empty / sentence-like / case-duplicate never enter) + every new term registered PENDING (`created:<call_site>`); JurisdictionRuleSet and bookkeeping types are NOT gated (their keywords are routing keys / system tags) |
| R29 | **Keyword validation** (`keyword_loops` settlement actor → `keyword_settle::settle_once` → `validate_terms`) | zero-shot (batched, asymmetric) | validates stored terms and new expansion candidates; kill switch `OZONE_KEYWORD_SETTLE=off`; strict search `OZONE_KEYWORD_REQUIRE_VALIDATED`, ablation `OZONE_KEYWORD_IGNORE_EXPANSION` (both default off) |
| R30 | **Rule gate** (after the blueprint draft, before `audit_rule_refs` → `judge_rule_applicability` (applicability + step→rule fidelity in ONE call, 2026-11-10 moved); rule text via `rules_text_for` at the four post-AMT prompt sites) | zero-shot (batched, asymmetric) | replaces the end-of-Stage-6 E14; verdicts in `state.rule_verdicts` + provider manifest rows |
| R31 | **Refinement notification** (`stages::judge_rule_applicability` → `meta_loop::record_refinement_candidate`) | free | durable `methodology_refinement_candidates.jsonl`: every rule judged inapplicable = a methodology that is too broad (refinement loop input; never a `new`/`modify` gap) |
| R32 | **Marketplace publish / pull / fetch / import** (`bridge (drains every creation event already available) → network::on_items_announced → publish_batch_to_marketplace` (ONE listing write, ONE push per batch); `network::spawn_marketplace_exchange`; `orchestrator::peer_import::spawn_peer_import_loop`; `POST /marketplace/offers/decide`) | free (no model call) | notify (one push, names nothing) → pull listing → operator decides → fetch in chunks → re-verify hash → import a methodology through the create path / stage a jurisdiction file / record approval for tools; all opt-in via `auto_broadcast_sync` |
| R33 | **The network door + offer review** (`network` swarm handler → `sanitize_marketplace_message` → `peer_marketplace_inbox.jsonl` / `peer_marketplace_rejected.jsonl`; `offer_review::spawn_offer_review_loop` → `peer_offer_reviews.json`; both exposed by `GET /marketplace/overview` as `peer_offers[].review`, `exchange.refused_inbound`) | door: free · review: zero-shot (advisory) | nothing not allowed is accepted: only methodology / jurisdiction_rule_set / pipeline / tool, strict shapes, scope markers stripped from peers' keywords, unknown fields dropped; the review only RECOMMENDS |
| R34 | **Work-order tree** (`work_order::materialize_work_order` after task creation in `stage_6_to_8_execute_steps`; `work_order_step_done` on both step-success paths; `work_order_after_replan`; `work_order_walk_done`; `GET /order/work/<task_id>`) | free | the walk's structure as `wo_*` task records (AMT branches, steps, capability-gap checklists); completion = all items done + all children complete, completed is final; items ticked only by the real events (STATUS §K1); PARSE-ONLY |
| R35 | **Refinement action** (`meta_loop::act_on_refinement_candidates`, pass step 1d) | zero-shot (batched vote, strength-N, unanimity) + one draft call per split | splits a rule that is a different concern into a focused methodology via pipeline 12 Create; records `methodology_refinement_actions.jsonl`; never edits the original; kill switch `OZONE_REFINEMENT_ACTION=off`; PARSE-ONLY |
| R36 | **Read endpoints for the UI (2026-11-10, no model, no write)**: `GET /capability/gaps` (every row of `capability_gaps.json`, any status; `count`, `by_status`), `GET /methodology/refinement` (`methodology_refinement_report.json` or `{}`, the parsed lines of `methodology_refinement_actions.jsonl` read live, and the candidate-line count), `/task/list` + `/task/get` now carry `kind`, `parent_task_id`, `rerun_of` | free | read-only views of ledgers the walk / meta loop already write; nothing derived or scored; `grpc/mod.rs` (`get_capability_gaps`, `get_methodology_refinement`, `TaskInfo`) — EDITED, PARSE-ONLY |
| R37 | **Tool creation on need** (`stages::close_capability_gaps` → `create_tool_for_capability` → `tool_creation::{parse_spec, write_artifact}` → `mcp::register_global` → `tool_binding::mark_gap_tool_created` → `network::publish_created_tool` → bridge `on_items_announced`) | zero-shot (ONE call, `capability_tool_draft`) | creates `<data_dir>/created_tools/<name>/` + a stdio registry row (nothing executed by the host); gap → `satisfied` with provenance; work-order `tool`/`register` ticked by `tool_created`/`tool_registered`; the finding is published as a manifest (name + body hash + keywords, item_type `tool`) while `marketplace_publish` is on. PARSE-ONLY |
| R38 | **System call graphs** (host boot → `orchestrator::call_graph::ensure_system_call_graphs` creates/refreshes one `Derived` graph container per graph from `assets/call_graphs/system_call_graph.json`; `GET /callgraph/system`, `GET /callgraph/live?task_id=\|trace_id=\|latest=1`) | free, no model | boot pass: idempotent (ids + body hashes in `<data_dir>/call_graphs/containers.json`, body in `<data_dir>/call_graphs/<key>.json`, container via the normal create door so R27/R28 capture and gate apply); endpoints are read-only (the asset; `capture/call_spans.jsonl` + `walk_metrics.jsonl` streamed by exact trace id, no row cap). The asset is generated by `tools/call-graph/extract_system_call_graph.mjs` (`--check` detects drift); the host never rewrites it. A call row is written when the call finishes, so an in-flight call is not in the live view. PARSE-ONLY (2026-11-11) |
| R39 | **Operator retry of a failed child** (`POST /task/retry {task_id}` → `TaskManager::retry_task`; `POST /task/cancel` accepts a failed/interrupted child) | free, no model | a retry is a NEW task under the SAME parent walk (its completion gate waits on it); a cancelled failed child no longer blocks the walk (2026-11-11) |
