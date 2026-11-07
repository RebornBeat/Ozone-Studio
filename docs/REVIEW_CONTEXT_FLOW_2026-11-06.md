# Context flow review — every context type, its flow, windowing/aggregation, span-out, limits, model-agnostic status (2026-11-06)

Operator: windows, relation windows, aggregation, context traversal — no limits, size by content/model; work per call OR as aggregate; span out where it applies and say where it does not; identify how context flows for every type and whether it is model-agnostic. Base = `docs/CONTEXT_REGISTRY.md` (S1–S12, wiring matrix, mixing map), `SPAN_OUT_AND_RESUME_2026-11-02.md`, `TOP_DOWN_REVIEW_GUIDE` §9/§10, `CONTEXT_BUDGET_SERVICE`, plus a fresh code sweep for literal limits in context paths (files listed in §4). **Verified** = read in code this pass; **doc** = taken from the registry docs, not re-verified.

## 1. The model-agnostic rule (what "correct" means here)
Work is sized by the model that READS the call: `resolve_context_limit` → Path1 (>8,192: whole-chunk) vs Path2 (≤8,192: granular loops); Stage 2 chunk = `model_context_limit/4` tokens; AMT builders are tiers of one process (layer-by-layer = many small calls, graph-native = fewer richer calls); smaller models LOOP MORE, the full context is always delivered in portions. Per call → window-fit one prompt; aggregate → chunk, call per chunk, merge (any-Decline-wins, union, any-covers). Nothing is dropped from the graph by a prompt view (trims only change the view and are recorded). Constants are allowed ONLY as retry counters or explicit host runaway guards that are reported when hit.

## 2. Context types — flow, status, model-agnostic?
| # | Context type | Produced → consumed | Carrier | Size handling | Span-out? | Model-agnostic |
|---|---|---|---|---|---|---|
| 1 | **Request text** (chunks, sections/paragraphs/sentences, modality detections) | Stage 2 text pipeline → AMT, step context | chunk graphs + carry state machines | chunk = `limit/4` tokens (**verified** mod.rs:2376); per-prompt `fit_chunk_to_window` records trims; Path1/Path2 by size | no (direct) | ✅ sized by model. ⚠ no per-size reuse (placeholder content hash — W12) |
| 2 | **Project AMT (S1) / request AMT (S2)** | Stage 5 → blueprint, gate, re-expansion | containers + `amt_p*.json` | two builders by tier; re-expansion budget `window/8` (**verified** amt_loop.rs:745) | **yes** (re-expansion, step start) | ✅ ; ⚠ `MAX_CANDIDATES_PER_PASS=12` (batch, leftovers next pass — OK pattern); candidate log evicts at 500 (warned) |
| 3 | **Blueprint (S3)** | Stage 6 → steps, simulation | container + `state.blueprint_steps` | full render in prompts | via step span-out | ✅ |
| 4 | **File / modality graphs (S4)** + fan-out digests | Stage 2/4b, step pipelines → span-out, context | ModalityGraph containers, `graph_fanout::digest_graph` | digest lines packed to the span-out budget (`lines_total` vs `lines_packed` reported); `sequence_head` shows 12 (a display field of the digest) | **yes** | ✅ ; ⚠ 16 modalities' relation windows are COUNT-based (30 nodes) not token-based; ⚠ ~75 prompt lists still first-N (REVIEW_ALIGNMENT) |
| 5 | **Jurisdiction (S5)** | Stage 0 → blueprint, gate | `state.jurisdiction_gate_result` | gate sees text WHOLE (J8 live-tested) | no | ✅ |
| 6 | **Methodology rule text (S6)** | Stage 3 → blueprint, compliance check | containers + content files | whole; 7 re-read sites (perf, see REVIEW_OPTIMIZATION) | no — keyword gather only | ✅ ; sufficiency review (NEW) CHUNKS gathered methodologies to the reading model's window and merges |
| 7 | **Simulation (S7) / consciousness gate (S8)** | Stage 4/8 | state | decision_review chunks by window (`(w*4/3).max(1500)`, **verified** decision_review.rs:395), any-Decline-wins | no | ✅ |
| 8 | **Coordination events (S9)** | /SharedContext → context_aggregation layer | containers | char budget — **was `token_budget` used as chars (4× too small); FIXED now ×4** | no | ✅ after fix |
| 9 | **Step context** (context_aggregation ForStep) | stages 9–11 | seeds + traversal + `known_seed_ids` | token budget; seeds uncapped (audit #8) | **yes** (`span_out_for_step`) | ✅ ; ⚠ per-seed traversal `depth 2 / results 10` literal (below) |
| 10 | **Chat history** | `recent_chat_context` → step context | chat turn files | **last 6 turns** (`OZONE_CHAT_CONTEXT_TURNS`, literal count), each turn FULL text; reads every turn file each time | no | ⚠ count, not size: should be "newest turns until the reading model's budget" |
| 11 | **I-Loop / assistant signals** | signals → reflection / digest | ledger of dated signals | I-Loop view `(context_length/4).max(500)` (**verified**), span-out `budget/3`; data kept, time-aware | **yes** (I-Loop) | ✅ |
| 12 | **Zero-shot / decision-review / ledger captures (S10–S12)** | every call → stores | jsonl | append, no cap; ledger corrupted by concurrent appends (63/5,797) | no | n/a (observability) — ⚠ integrity |
| 13 | **Missing-capability / tool-health / binding-proposal records (NEW)** | Stage 3/provider steps → marketplace, meta | json files | **500-row caps REMOVED (mine, edited now)** | no | ✅ |
| 14 | **Methodology reviews & gaps (NEW)** | Stage 3 → meta loop | `methodology_reviews.json`, `methodology_gaps.json` | no cap; review CHUNKS methodologies by the reading model's window | no (see §3) | ✅ |
| 15 | **Provider/tool outputs** | MCP call → post-call hook → graph | `McpResult/McpEntity` graphs (`called_from`) | `mcp_graph`: refuses untrusted graphs > 5,000 nodes / 20,000 edges; labels cut at 200 chars | no | ⚠ refusal-by-size on tool output (guard); should ingest in chunks |
| 16 | **Model-routing context (NEW)** | ledger/catalog/quota/limiter → routing | files | prompt binary re-reads ledger+catalog per call (perf) | no | ✅ |
| 17 | **Host log (NEW)** | tracing → file (persistent) | `zsei_data/logs/host.log` | rotate >256 MiB, never truncate | no | n/a — link to meta by beacon (design in REVIEW_OPTIMIZATION) |

## 3. Span-out: where it applies, where it does not
**Applies (verified callers):** step start (`stages.rs:2789` → `span_out_for_step`, budget `limit/4`), AMT re-expansion (`amt_loop.rs:745`, `window/8`), I-Loop (`i_loop.rs:257`, `budget/3`), `/context/span_out` (`grpc/mod.rs:950`, **default 8,000 tokens literal when the caller gives none**).
**Does not span out (and why it is right or open):** Stage 2 (reads the request itself — right); Stage 3 methodology gather (keyword search; **open:** the sufficiency review judges only keyword-gathered methodologies — a span-out from them to RELATED methodologies would show neighbours the keyword miss; candidate, not built); meta-loop draft (keywords/topics only — the "thinnest judgment" consumer in CONTEXT_REGISTRY §6, still open); assistant digest (findings list); decision-review (chunked task summary + traversal-assembled review context); compliance check (rule text + step output, bounded by design); yes/no votes (the question only, by design); tool routing.
**Algorithm guards:** seeds uncapped (removed this session: take(40) + 12-keyword floor), token budget stops the walk, `DEFAULT_FETCH_CAP = 400` is a host runaway guard (env `OZONE_SPAN_OUT_MAX_FETCH`, hit reported via `fetch_cap_hit` + `frontier_left`). ⚠ it is a CONSTANT while the token budget scales with the model: on a 1M-window model the budget cannot fill before 400 fetches. Model-agnostic form = derive the guard from the budget (e.g. budget ÷ smallest useful node size) and keep the env override.

## 4. Limit audit of the context paths (files swept: span_out, graphs, amt, amt_loop, i_loop, meta_loop, stages, lane_split, awaiting, graph_fanout, decision_review, amt_candidates, response, context_budget/mirror/needs, assistant, zsei query/search/traversal, context_aggregation, mcp_graph, tool_routing/binding, jurisdiction)
| Literal | Where | Class | Disposition |
|---|---|---|---|
| `MAX_ROWS 500` ×2 | tool_binding (gaps/proposals) | DATA CAP (mine) | **REMOVED now** |
| `max_chars = token_budget` | context_aggregation coordination layer | UNIT BUG | **FIXED now (×4)** |
| `DEFAULT_FETCH_CAP 400` | span_out | host guard, constant vs scaling budget | **EDITED (L4): scales with budget (floor 400), env override kept** |
| `TRAVERSAL_MAX_RESULTS 10`, depth 2 | context_aggregation traversal | content cut per seed (documented "class D host-work bound") | **EDITED (L3): results = budget/50 (floor 10); depth/latency guards stay** |
| `MAX_NODES 5000 / MAX_EDGES 20000 / label 200 / attr 64 KiB` | mcp_graph | refusal-by-size on tool output | **EDITED (L2): digests any size; per-source edge batches; full label/attrs in content file** |
| chat `last 6 turns` | chat_store | count not size | **EDITED (L5): newest-first until limit/8 tokens** |
| `MAX_CANDIDATE_LOG_ENTRIES 500` | amt_candidates | evicts oldest handled, then (warned) unhandled | **EDITED (L6): handled archived, unhandled never evicted** |
| `sequence_head take(12)` | span_out digest | display field of a digest | left (lines are packed to budget separately) |
| `/context/span_out` default 8000 | grpc | literal default budget | **EDITED (L4): default = window/4 (8000 only if unknown)** |
| `MAX_*_ATTEMPTS / RETRIES (2,10)`, `MAX_LOOP_ITERATIONS 1000`, `MAX_GAPS_PER_PASS 12`, `MAX_CANDIDATES_PER_PASS 12` | graphs, meta/amt loops, stages | retry counters / per-pass batches with leftovers carried, runaway guards | left (policy, not content) |
| `chars().take(200/500/60)` | decision_review labels, response probe, amt_loop error text | labels/previews of already-stored content | left (display only) |
| ~75 `.take(N)` prompt lists + 63 `chars().take(N)` | modalities | MODEL INPUT CUTS | captured — per-site windowing (REVIEW_ALIGNMENT, REVIEW_WINDOWING W6) |
Not swept (named so nothing is silently assumed clean): `src/consciousness/*` beyond assistant, `src/network/*`, `src/zsei/storage.rs`, `ui/`, `tools/*-mcp`, the cross-modal pass logic ("until 5 stable passes" is a stability criterion, not a size limit), `lane_split.rs` (read in the optimization review only).

## 5. Status summary
- ✅ model-sized and traversal-aware: request chunks, path selection, AMT re-expansion, I-Loop, decision-review chunking, span-out budget, jurisdiction (whole), sufficiency review (chunked).
- ⚠ literal-limited (captured): chat turns (6), traversal results (10), span-out fetch guard (400) and default budget (8000), MCP graph size refusal, candidate log eviction, relation windows (NOW token-based, L1), ~140 prompt cuts.
- ⚠ no per-size reuse: re-analysis each time a content/size pair recurs (W12).
- Edited this pass: tool_binding caps removed; context_aggregation unit fix. Everything else here is captured only.

**Update (late 2026-11-06):** L1–L6 edits above are UNCOMPILED; see REVIEW_WINDOWING §5 and §6 (remaining edits).
