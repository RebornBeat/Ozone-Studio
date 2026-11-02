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
| R7 | `mcp::call_global` (tool routing, sensor tick, approved assistant proposals) | in-process | G10 + **G4 added 2026-11-02** (these skipped jurisdiction) | usage ledger + gate verdict rows | ✅ |
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
