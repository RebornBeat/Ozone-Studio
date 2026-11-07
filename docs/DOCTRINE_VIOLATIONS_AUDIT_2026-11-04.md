# Doctrine violations audit — where the rules are broken (2026-11-04)

Read-only audit (fork). Rules: **R-CAP** no cap on content (budgets follow the reading model's window; no fixed char/byte/line/item counts), **R-EMPTY** never turn an empty/missing/unparseable response into `{}`/defaults, **R-GATE** a gate must see the WHOLE input, **R-FAB** no fabricated defaults presented as real, **R-SEARCH-FLOOR** no first-N floors presented as selections. Classes: **A** content cut that changes what a model/gate/store sees (must fix) · **B** coverage bound that silently drops items from a pass (must fix / make batch-complete) · **C** identifier/hash/display-preview/log bound (acceptable) · **D** resource/safety guard on host work with an honest report (acceptable) · **E** tests. `OK` = verified compliant. Method: regex sweep of `src/**/*.rs`, `assets/pipelines/**/*.rs`, `tools/**/*.mjs|py`, `ui/src/**/*.ts(x)` (≈110 `chars().take`, 74 `MAX_*`, 64 `truncate`, 141 `slice(0,N)`, 32 `max_tokens` literals, all `{}`/default fallbacks) + manual context reads of every non-trivial hit. Line numbers were taken from the tree at 2026-11-04 and will drift; the greppable snippet is given too.

## 0. Headline

- **Most dangerous (class A, security/model input):** (1) `check_call_gate` 4,000-char window — FIXED today; (2) attached-file cut at 200 KB with a panicking byte `truncate` (`orchestrator/mod.rs:2215`); (3) methodology gather searches only the first 20 keywords (`mod.rs:2524`); (4) relationship inference sees 30 nodes × 80 chars in every modality (`infer_semantic_relationships_validated`); (5) `load_methodology_rules_text` 5 rules / 3 heuristics; (6) legacy gate-record migration invents `Proceed` / ethical score 1.0 (`consciousness/store.rs:1283`); (7) Anthropic path default `max_tokens` 4096 (`prompt/main.rs:637`); (8) terminal-mcp stdout cut at 100 KiB with no flag; (9) browser-mcp typed text cut at 5,000 chars; (10) MCP servers destroy oversized request sockets with no reply (security-relevant: a gate-bearing body above 1 MiB is never gated and the host reports 'tool unreachable').
- The old fixed-count context path in `context_aggregation` (4 seeds / 8 containers / 1,600-byte coordination layer) still runs beside span-out.
- `parse_json_object` (`{}` on parse failure) is dead code kept only by its test — delete before someone reuses it.

## 1. Counts

| Class | Findings (grouped rows) |
|---|---|
| A | 26 |
| B | 14 |
| C | 15 |
| D | 14 |
| E | 1 |
| OK | 3 |

| Subsystem | A | B | C | D | E | OK |
|---|---|---|---|---|---|---|
| mcp/gates | 4 | 0 | 0 | 1 | 0 | 3 |
| orchestrator | 12 | 6 | 7 | 6 | 0 | 0 |
| pipelines/prompt | 2 | 1 | 1 | 0 | 0 | 0 |
| pipelines/context_aggregation | 1 | 2 | 0 | 1 | 0 | 0 |
| pipelines/browser_navigation | 1 | 0 | 0 | 0 | 0 | 0 |
| pipelines/other | 0 | 0 | 1 | 1 | 0 | 0 |
| modalities | 2 | 2 | 0 | 2 | 0 | 0 |
| tools | 4 | 0 | 2 | 3 | 1 | 0 |
| ui | 0 | 3 | 4 | 0 | 0 | 0 |

(rows group same-pattern hits; e.g. the `k.truncate(12)` row stands for 24 modality files, the `take(30)/take(80)` row for ~20.)

## 2. Findings by subsystem (class-A/B ranked by severity: S1 gate/model input → S2 stored/consumed content → S3 discovery/coverage → S4 display)

### mcp/gates

| Sev | Class | Rule | Location | What | Recommended fix |
|---|---|---|---|---|---|
| S1 | A | R-GATE R-CAP | src/mcp.rs:161 (FIXED 2026-11-04) | `text.chars().take(4000)` into the jurisdiction call gate | Done: whole text. Add test J8 (tools/e2e/jurisdiction.mjs). |
| S1 | A | R-CAP | src/orchestrator/tool_routing.rs (removed 2026-11-04): query[..200], MAX_RESULT_CHARS 2000, MAX_BLOCK_CHARS 4000, MAX_TOOLS_PER_STEP 2, `.take(12)` words | tool routing caps | FIXED (whole step text, window-fit by caller, relevance group). Add a unit test that a 10 000-char step text reaches the tool whole. |
| S2 | A | R-CAP R-GATE | tools/{web,dna,connectome,security,re,terminal,document,browser,browser-farm,p2p-bridge,threed}-mcp/server.mjs (`size > 1 * 1024 * 1024 → req.destroy()`; cerebrix/visual/code 20 MiB) | oversized request body: socket destroyed, no reply | Return HTTP 413 + JSON `{success:false,error:'request body over N bytes'}`; make N env-tunable; the host then reports an honest error instead of 'tool unreachable'. (Any gate-bearing input larger than the limit is silently never gated downstream.) |
| S2 | A | R-CAP | src/network/mod.rs:1212-1214 `DIALOGUE_MAX_BODY_CHARS=4000` | `turn.body = turn.body.chars().take(4000).collect()` — peer dialogue silently truncated | Reject the turn with an explicit error (never relay a silently cut body) or size by the receiving model's window; record the trim. |
| S- | OK | R-GATE | src/orchestrator/jurisdiction.rs:737 `step_gate_check` + src/orchestrator/tool_routing.rs `step_verdict` | gate runs on the whole `step_description` (no cut) | Verified whole-text. Caveat: only the step TEXT is gated, not prior-step outputs/context injected into the step prompt — decide if that is intended. |
| S- | OK | R-GATE | src/grpc/mod.rs ~1072 (`/pipeline/execute`) | `serde_json::to_string(&req.input)` → `check_call_gate` (now whole text) | Verified whole input. |
| S- | D | R-CAP | src/mcp_graph.rs:49-54 (MAX_NODES 5000, MAX_EDGES 20000, MAX_LABEL_CHARS 200, MAX_KIND_CHARS 40, MAX_ATTR_BYTES 64K), :516 `take_chars(&text,4_000)`, :902/907 truncate edges | host-side guards when persisting a tool's `graph` block | Reported in `notes` (:631 shows first 20 notes only — C). Labels cut at 200 are display text (C). Keep; ensure `notes` is surfaced in the call result. |
| S- | OK | R-GATE | src/mcp.rs / src/orchestrator/entry_gate.rs verdict rows | gate verdict rows record reason (no content cut) | — |

### orchestrator

| Sev | Class | Rule | Location | What | Recommended fix |
|---|---|---|---|---|---|
| S1 | A | R-CAP (+ crash) | src/orchestrator/mod.rs:2215-2230 `MAX_FILE_ANALYSIS_BYTES=200_000` + `content.truncate(..)` | attached file cut at 200 KB before analysis; `String::truncate` PANICS if byte 200000 is not a char boundary | Remove the cut: analyse in windows/chunks to completion (batch-complete) or fail the file loudly; at minimum truncate on a char boundary and record `file_truncated` in the run record. |
| S1 | B | R-CAP R-SEARCH-FLOOR | src/orchestrator/mod.rs:2524 `state.keywords.iter().take(20)` → `search_by_keywords(.., Some("Methodology"))` | methodology GATHER (the heart of Stage 3) searches only the first 20 request keywords | Search with ALL keywords (batch the store query), or rank keywords and loop until the set is covered; record coverage. |
| S1 | A | R-CAP | src/orchestrator/amt.rs:2598/2608 `rule_limit/heuristic_limit` (load_methodology_rules_text uses 5/3) | `load_methodology_rules_text` limits to 5 rules + 3 heuristics; `_full` has no cap | Make every caller use `_full` + render_within_window; delete the limited variant (grep callers). |
| S1 | A | R-EMPTY | src/orchestrator/mod.rs:5440 `parse_json_object` → `unwrap_or_else(|_| json!({}))` | unparseable model JSON becomes `{}` | Only used by its own test (dead). DELETE it (and its test) so it cannot be reused; `parse_json_object_strict` is the live path. |
| S1 | A | R-FAB R-EMPTY | src/consciousness/store.rs:1283-1290 `migrate_hook_gate_decisions` (`ethical_score … unwrap_or(1.0)`, `decision … unwrap_or("Proceed")`) | migration of legacy gate records invents a Proceed decision with ethical score 1.0 when fields are missing | Skip/mark `unverified_migrated` instead of fabricating an approval; never default a gate decision to Proceed. |
| S2 | A/B | R-CAP R-SEARCH-FLOOR | src/orchestrator/stages.rs:733 `.take(15)` (blueprint reuse search), :603 `.take(8)` + :617 `.take(3)` + :639 `.take(8)` (prior-design context: 8 kw, 3 blueprints, 8 step descriptions each), :1304 `.take(8)` (branch keywords), :1680 `.take(8)` (simulation related containers) | first-N keyword/blueprint/step floors feeding reuse + prior design + simulation context | Use all keywords; fit prior-design text with render_within_window instead of 3×8; keep the 0.95 Jaccard reuse test on the FULL keyword set. |
| S2 | A | R-CAP | src/orchestrator/awaiting.rs:150,151,168,177,187 (subject 400, reason 800, resolution 800) | awaiting reason/resolution text cut — this is what a human reviews | Store whole; truncate only at display time in the UI. |
| S2 | A/B | R-CAP R-SEARCH-FLOOR | src/orchestrator/graphs.rs:480 `.take(15)` keywords, :487 `.take(8)` topics (graph-creation prompt), :842 `.take(12)` + :849 `.take(6)` (prompt-term seeds) | graph-creation prompt and seed discovery use first-N | Pass all keywords/topics, window-fit; batch seed search. |
| S2 | A | R-FAB | src/orchestrator/{mod.rs:2276,graphs.rs:162} `confidence … unwrap_or(0.5)` (math ParseExpression), src/zsei/hooks.rs:797,894,953 edge `confidence … unwrap_or(0.5)` | missing model confidence becomes a fabricated 0.5 on graph edges/parse results | Store `None`/absent (or `null`), never 0.5; consumers treat absent as unknown (span-out already defaults edge weight, not the stored value). |
| S2 | A | R-CAP | src/context_needs.rs:179 `raw.chars().take(400)` | context-needs extract cut at 400 chars | Window-fit instead of a fixed 400. |
| S2 | A | R-CAP | src/zsei/hooks.rs:605 `graph.nodes.iter().take(50)` ("Limit for prompt size") | graph summary prompt shows only 50 nodes | Window-fit the node summary (or use the new fan-out digest). |
| S3 | B | R-CAP R-SEARCH-FLOOR | src/orchestrator/amt.rs:3258 `.take(12)`, :3357 `.take(8)`, :3375 `.take(3)` (related blueprints), :3448 `.take(6)`, :3468 `.take(12)` (hop seeds) | AMT enrichment search/hops capped by count | Replace counts with score thresholds + window budget (as span-out does); record the cutoff. |
| S3 | B | R-SEARCH-FLOOR | src/orchestrator/amt.rs:2694-2739 `rank_methodologies(max_selected)` | selection policy bounds how many methodologies earn branch-discovery lanes (`usize::MAX` = all) | Default to ALL (documented switch exists); if a bound stays, record uncovered methodologies (already logged as UNCOVERED lanes). |
| S3 | A/C | R-CAP | src/orchestrator/amt_candidates.rs:122 `detail.take(200)`, actors.rs:576 `detail.take(600)` (coordination finding body), actors.rs:313 `take(80)`, actors.rs:822 `take(120)`, amt_loop.rs:541, instrumented.rs:91 `take(300)`, response.rs:275 `take(60)` probe | candidate/finding detail + log previews | actors.rs:576 puts a 600-char cut INTO a stored coordination note (A-lite): store whole. The rest are log/identifier previews (C). |
| S3 | A | R-FAB | src/consciousness/store.rs:1270 (`arousal … unwrap_or(0.3)`), :1339 (`relevance_score … unwrap_or(0.8)`) | defaults for missing emotional/relevance values | Use Option/absent; the dashboard shows 'unknown'. |
| S3 | B | R-CAP R-SEARCH-FLOOR | src/zsei/query.rs:654 `direct.iter().take(12)` + `neighbors_injected >= 12` | traversal neighbours injected: 12 seeds / 12 neighbours | Budget by tokens/score (span-out) instead of counts. |
| S- | C | R-CAP | src/orchestrator/stages.rs:2185 `cleaned_prompt.chars().take(200)` (awaiting subject) | awaiting-item subject label | Label only; keep (full prompt is on the task). |
| S- | C | R-CAP | src/orchestrator/decision_review.rs:518,527 (task_summary 200), :640 `cut=500` (jsonl capture) | review labels and capture previews | Previews in a capture log; label only. Verify the review itself chunks the WHOLE task text (chunk_idx/total_chunks — verified, batch-complete). |
| S- | C | R-CAP | src/orchestrator/mod.rs:3666,3822 `cut(.., 500/300)` (zero_shot_calls.jsonl response_preview) | capture preview field | Named `response_preview`; fine. (Full responses live in the S13 capture/model_calls stores — confirm.) |
| S- | D | R-CAP | src/orchestrator/amt_loop.rs:346 `MAX_CANDIDATES_PER_PASS=12`, src/orchestrator/meta_loop.rs:122 `MAX_GAPS_PER_PASS=12` | bounded waves; leftovers named and run next tick | Batch-complete by design (deferred, logged). With event-driven wakes ensure a follow-up wake fires while leftovers remain. |
| S- | D | R-CAP | src/orchestrator/amt_candidates.rs:23 `MAX_CANDIDATE_LOG_ENTRIES=500`, src/orchestrator/amt.rs:2675 `MAX_GAP_LOG_ENTRIES=500`, tool_binding.rs `MAX_ROWS=500` (gaps, proposals), src/monitor/mod.rs:16 `MAX_EVENTS=500` | observability log bounds drop the OLDEST rows | Acceptable log bounds; the I-Loop now sees dates. Note the candidate log drops history the I-Loop reasons over — consider archiving instead of dropping. |
| S- | D | R-CAP | src/orchestrator/span_out.rs:92 `DEFAULT_FETCH_CAP=400` + `.take(40)` keyword seeds (:437), :358 `.take(12)` sequence_head | host-work guard (reported as `fetch_cap_hit`), seed and display bounds | fetch cap is REPORTED; `.take(40)` keyword seeds is a B-lite (document); `sequence_head` is display. |
| S- | D | R-CAP | src/orchestrator/stages.rs:1269 `MAX_LOOP_ITERATIONS=1000`, :3353 `MAX_COMPLIANCE_RETRIES=2`, graphs.rs:554 `MAX_COMPLETENESS_RETRIES=2`, amt_loop `MAX_REEXPAND_ATTEMPTS=10`, meta_loop `MAX_DRAFT_ATTEMPTS=10` | retry/iteration safety bounds | Safety bounds, failures are recorded/paused. |
| S- | C | R-EMPTY | src/orchestrator/lane_split.rs:100-107 `parse_object` → `json!({})` | unparseable lane-split JSON becomes `{}`; the caller (:179) treats a missing `branches` as the failure branch | Acceptable because the consumer fails on absence; rename to `parse_object_or_empty` + comment, or return `Option`. |
| S- | C | R-EMPTY | src/orchestrator/stages.rs:3029 `sub_result.clone().unwrap_or_default()` | sub-step Err recorded as empty output | Followed immediately by the failure check (`success: is_ok()` → step fails); acceptable. |
| S- | C | R-EMPTY | src/orchestrator/decision_review.rs:474 `result.unwrap_or_default()` | after the all-models-unusable Err return | Guarded; acceptable. |
| S- | D | R-CAP | src/context_budget.rs:89 `trim_at_paragraph(max_chars)`; assets/pipelines/shared/context_assemble.rs:64 (same) | paragraph-boundary trim to a BUDGET passed by the caller | This IS the budgeted trim (records trims). Verify every caller derives `max_chars` from the window, not a literal. |
| S- | D | R-CAP | src/orchestrator/mod.rs:4679 `.take(3)` free API fallback models | fallback chain policy | Policy bound on provider attempts; recorded per call. |

### pipelines/prompt

| Sev | Class | Rule | Location | What | Recommended fix |
|---|---|---|---|---|---|
| S1 | A | R-CAP | assets/pipelines/general/prompt/main.rs:637 `"max_tokens": input.max_tokens.unwrap_or(4096)` (Anthropic path) | Anthropic requires max_tokens; 4096 default is a hidden response cap | Default to the model's real maximum output (from the model profile) and document; when `max_tokens` is only a hint (doctrine), pass the maximum, not 4096. |
| S3 | A | R-CAP | assets/pipelines/general/prompt/main.rs:1073 `input.max_tokens.unwrap_or(1024)` (local ONNX/py path), :1273 `.unwrap_or(512)` | local-path defaults | Derive from the loaded model window like -n (half the window). |
| S3 | B | R-CAP (indirect) | assets/pipelines/general/prompt/main.rs `request_timeout()` (new) | HTTP timeout derived from `max_tokens` (45 s + 1 s/10 tokens, cap 600 s) | See the max_tokens table: do NOT keep deriving from a caller hint; derive from prompt size + a floor of 90 s and the provider's observed latency. |
| S- | C | R-CAP | assets/pipelines/general/prompt/main.rs:580,893 `message.chars().take(300)` | error-message previews | Diagnostic previews. |

### pipelines/context_aggregation

| Sev | Class | Rule | Location | What | Recommended fix |
|---|---|---|---|---|---|
| S1 | B | R-CAP R-SEARCH-FLOOR | assets/pipelines/general/context_aggregation/main.rs:238 `MAX_TRAVERSAL_SEEDS=4`, :509 `MAX_RESOLVED_CONTAINERS=8`, `TRAVERSAL_MAX_DEPTH=2`, `TRAVERSAL_MAX_RESULTS=10` | the old fixed-count context path still runs beside span-out | Delete the fixed counts: seed/resolve by score within the token budget (span_out_context already supplies the window-filled context); keep the old path only as a fallback marked as such. |
| S1 | A | R-CAP | assets/pipelines/general/context_aggregation/main.rs:618 `MAX_CHARS=1600` (coordination layer, byte-based `layer.len()`), :595 body `take(240)` | coordination context cut to 1600 bytes / each note 240 chars | Fit to a token budget (share of the window); truncate on char boundaries. |
| S3 | D/B | R-CAP | assets/pipelines/general/context_aggregation/main.rs:325 `truncate_to_budget`, :720 `keyword_list.truncate(12)` (+ `>= 12` loop) | token-budget trim; keyword list capped at 12 | truncate_to_budget is budgeted (D). The 12-keyword cap is a B: use all keywords. |

### pipelines/browser_navigation

| Sev | Class | Rule | Location | What | Recommended fix |
|---|---|---|---|---|---|
| S2 | A | R-CAP | assets/pipelines/general/browser_navigation/main.rs:267-269 | `innerText.slice(0,100000)`, links `slice(0,100)`, images `slice(0,50)` | Return everything with explicit paging/`truncated:true` + counts. |

### pipelines/other

| Sev | Class | Rule | Location | What | Recommended fix |
|---|---|---|---|---|---|
| S- | D | R-CAP | assets/pipelines/general/{traversal_ml:248,359,406,474, code_analysis:1642, text_analysis:743, code_analysis:1321 (keywords.truncate(20)), voice:344,356} | `max_results/limit` truncations | Caller-provided limits on result lists (paging semantics) — D. code_analysis:1321 `keywords.truncate(20)` is B-lite. |
| S- | C | R-CAP | assets/pipelines/shared/capture.rs:80 `cut(.., n)` | capture previews | Preview fields. |

### modalities

| Sev | Class | Rule | Location | What | Recommended fix |
|---|---|---|---|---|---|
| S1 | A | R-CAP R-SEARCH-FLOOR | assets/pipelines/modalities/{3D:1541,BCI,CAD,IMU,biology,control,depth,electromagnetic,geospatial,haptic,hyperspectral,kinematics,network,proteomics,radar,sonar,sound,thermal}/main.rs `nodes.iter().take(30)` + `n.content.chars().take(80)` in `infer_semantic_relationships_validated` | relationship inference prompt sees only the first 30 nodes, 80 chars each | Chunk the node list through the prompt to completion (batch) and send whole node content (window-fit) — otherwise cross-node relations beyond node 30 are never found. (The shared `semantic_relations` helper is the right place.) |
| S2 | A | R-CAP | assets/pipelines/modalities/uidocument/src/html.rs:85-86 `MAX_TEXT=4000` text-node cut (silent), a11y.rs:14,63,196,245 `NAME_LIMIT=300` (silent), a11y_json.rs MAX_TEXT, android.rs MAX_TEXT 4000 | per-node text silently cut (graph-level guards DO report in `notes`) | Record `text_truncated` per node/in notes (same mechanism as the input-size note at html.rs:153) or keep the full text. |
| S3 | B | R-CAP R-SEARCH-FLOOR | assets/pipelines/modalities/*/main.rs (3D, BCI, CAD, IMU, audio, biology, chemistry, control, depth, dna, eeg, electromagnetic, geospatial, haptic, hyperspectral, image, kinematics, network, proteomics, radar, sonar, sound, thermal, video: `k.truncate(12)`; text: `keywords.truncate(20)` :5029,:5058) | container keyword set capped at 12 (20 for text) — keywords drive `search_by_keywords` discovery | Keep all distinct keywords (dedup + rank, no count cut); at most bound by the container size budget. |
| S3 | B | R-CAP (intent) | assets/pipelines/modalities/text/main.rs:2758,3213,3505,3767,4013,4307,4345,4433,5580,5843,6289,6813; math:1951 `"max_tokens": 100…2000` | fixed `max_tokens` literals at ~13 call sites | See table; they are hints (API cap ignored) but feed `request_timeout()` and local paths. |
| S- | D | R-CAP | assets/pipelines/modalities/uidocument/src/{css.rs,rules.rs,lib.rs,graph.rs,svg.rs,html.rs}: MAX_SHEET_BYTES 2 MiB, MAX_RULES 20k, MAX_CASCADE_WORK 30M, MAX_DOM_NODES 20k, MAX_HTML_BYTES 4 MiB, MAX_NODES 5000, MAX_EDGES 20000, MAX_DEPTH, MAX_PATH_CHARS 200k … | untrusted-input DoS guards with `notes` | Safety guards on UNTRUSTED markup; reported. Keep, but the page-level `MAX_HTML_BYTES 4 MiB` should be env-tunable. |
| S- | D | R-CAP | radar/real.rs:201 (200 detections), thermal/real.rs:174 (2000 regions), hyperspectral/real.rs:396 (500 hits), proteomics/structure.rs:488-490, chemistry/molparse.rs:1035-1036, geospatial/geometry.rs:28-29, video/main.rs:940-941, audio/main.rs:1084, image/main.rs:1433, 3D/glue.rs:12 | signal-processing output/resource bounds | Each emits a note ('N found; M emitted'); verified for radar/thermal. Keep. |

### tools

| Sev | Class | Rule | Location | What | Recommended fix |
|---|---|---|---|---|---|
| S1 | A | R-CAP | tools/browser-mcp/server.mjs:362 `String(input.text).slice(0, 5000)` (Input.insertText) | typed text silently cut at 5000 chars | Type the whole text (chunked insertText) or error when over a stated limit. |
| S1 | A | R-CAP | tools/terminal-mcp/server.mjs:148-157 `cap = 100 KiB` (`stdout.slice(0,cap)`) | command output cut with no flag | Add `stdout_truncated:true` + total bytes; offer paging/`tail`. |
| S2 | A | R-CAP | tools/browser-mcp/server.mjs:243 page `text … slice(0,3500)`, :234 `.slice(0,150)` elements, :239 value 60, :242 labels 80 | perception snapshot cut (3.5 KB text, 150 elements) | Return `truncated:true` + counts and support paging (`snapshot {offset}`), or budget by window. |
| S2 | A | R-CAP | tools/git-mcp/server.mjs:62 `return out.slice(0, 500)` (file_diff) | diff lines capped at 500 silently | Return all lines or `truncated:true` + total. |
| S- | D | R-CAP | tools/browser-mcp/server.mjs:318 `capText` OUT_CAP 60 KiB (+ browser-farm :122), document-mcp:202 `rows.slice(0,500), truncated` | output caps WITH an explicit `truncated:true` flag | Honest; consider paging. |
| S- | D | R-CAP | tools/android-mcp/server.mjs:149,150,364 (labels 200/80), :175 `edges.slice(0,GRAPH_MAX_EDGES)` (`bounds` reported), browser-mcp:285,286,311 (same) | graph labels/edges with reported bounds | Reported in `bounds`. |
| S- | C | R-CAP | tools/{cerebrix,dna,code,visual,web,code-graph,connectome,threed,mesh,chemistry}-mcp `…slice(0, 200..500)` on stderr/stdout in ERROR messages | error text previews | Diagnostics. |
| S- | D | R-CAP | tools/security-mcp/server.mjs:245 `processes: all.slice(0, limit)` | caller-supplied paging limit | Paging. |
| S- | C | R-CAP | tools/ozone-cli/ozone.mjs:113,123 `name.slice(0,60)` | table column width | Display. |
| S- | E | R-CAP | tools/e2e/*.mjs `slice(0, N)` previews | test output previews | Tests. |

### ui

| Sev | Class | Rule | Location | What | Recommended fix |
|---|---|---|---|---|---|
| S3 | B | R-CAP R-SEARCH-FLOOR | ui/src/views/search/UnifiedSearch.tsx:60 `ids.slice(0,60)`, :102/:119 `hits.slice(0,30)` | search coverage capped (60 containers resolved, 30 results) with no 'more' affordance | Page ('load more') and show total counts. |
| S3 | B | R-CAP | ui/src/graphkit/layout3d.ts:83-85 `MAX_GRAPH_NODES=2400, MAX_CONTAINER_NODES=300, MAX_FILE_NODES=300` + :192,:289,:329 slices | 3D layout shows a subset of the graph | Level-of-detail / on-demand expansion with a visible 'N of M shown' (verify one exists); the fan-out lens must not hide the entries. |
| S4 | C/B | R-CAP | ui/src/panels/enginesRegistry.tsx:89 `nodes.slice(0,200)`, views/hierarchy/AmtTreeView.tsx:421 `chunkMatches.slice(0,20)`, graphkit/GraphNativeWorkspace.tsx:297,490,564 (14/8/14), graphkit/lens.tsx:57 `nodes.slice(0,50)`, files/FileReferenceViewer.tsx:193 `relations … slice(0,20)` | list renderers show first N | Display bounds; add 'show more' where data is hidden (FileReferenceViewer:193 resolves only 20 relation targets — B). |
| S4 | C | R-CAP | ui/src/views/files/CodeViewer.tsx:142,347 `MAX_LINE_CHARS=10000`, views/capture/ConfettiViewer.tsx:359 `slice(0,140)`, :344, captureStatus.tsx:253 `clip`, imageNodes/codeNodes renderers (97/117 char kv) | viewer truncation with ellipsis | Display with '…'; CodeViewer cuts a line at 10 000 chars — add a 'show full line'. |
| S- | C | R-CAP | ui/src/components/ThemeArea.tsx:202 `slice(0,50)`, McpActivityIndicator:81 `0,8`, OrchestrationStatusPanel:168 `0,3`, CreationActivity:54 MAX_VISIBLE, coordination/ChatNotifications MAX_RETAINED/MAX_SHOWN, WorkspaceStatusStrip:99 `0,8` | ephemeral toast/activity rings | UI ephemera, not data. |
| S- | C | R-CAP | ui/src/components/MarketplacePanel.tsx:166 `p.tools.slice(0,8)` (+N chip), :356 `example_step.slice(0,120)` | display | Shows '+N'; example_step could expand. |

## 3. `max_tokens` literals and `request_timeout()`

| Location | max_tokens | Call site | derived HTTP timeout = 45 s + v/10 (cap 600) |
|---|---|---|---|
| src/consciousness/assistant.rs:648 | 350 | assistant check-up digest | 80 s |
| src/orchestrator/decision_review.rs:438 | 300 | consciousness decision review (per chunk) | 75 s |
| src/orchestrator/response.rs:308 | 800 | final response render | 125 s |
| src/orchestrator/meta_loop.rs:305 | 800 | methodology draft | 125 s |
| src/orchestrator/graphs.rs:538 | 500 | graph completeness check | 95 s |
| src/orchestrator/mod.rs:5025 | 300 | (a helper call) | 75 s |
| src/orchestrator/amt.rs:1393 | 500 | AMT detail/domain/synthesis lanes | 95 s |
| src/orchestrator/amt.rs:1722 | 700 | AMT detail/domain/synthesis lanes | 115 s |
| src/orchestrator/amt.rs:2084 | 150 | AMT detail/domain/synthesis lanes | 60 s |
| src/orchestrator/amt.rs:2884 | 900 | AMT detail/domain/synthesis lanes | 135 s |
| src/orchestrator/amt.rs:3833 | 200 | AMT detail/domain/synthesis lanes | 65 s |
| src/orchestrator/amt.rs:3899 | 400 | AMT detail/domain/synthesis lanes | 85 s |
| src/orchestrator/stages.rs:1035 | 1000 | blueprint / simulation / step / compliance | 145 s |
| src/orchestrator/stages.rs:1857 | 800 | blueprint / simulation / step / compliance | 125 s |
| src/orchestrator/stages.rs:2663 | 200 | blueprint / simulation / step / compliance | 65 s |
| src/orchestrator/stages.rs:3342 | 150 | blueprint / simulation / step / compliance | 60 s |
| src/orchestrator/amt_loop.rs:797 | 400 | AMT re-expansion deepen | 85 s |
| src/grpc/mod.rs:2054 | 2048 | grpc-side prompt helper | 249 s |
| assets/pipelines/modalities/math/main.rs:1951 | 600 | math llm_zero_shot | 105 s |
| assets/pipelines/modalities/text/main.rs:2758 | 600 | text normalisation validations (stage 2) | 105 s |
| assets/pipelines/modalities/text/main.rs:3213 | 400 | text normalisation validations (stage 2) | 85 s |
| assets/pipelines/modalities/text/main.rs:3505 | 300 | text normalisation validations (stage 2) | 75 s |
| assets/pipelines/modalities/text/main.rs:3767 | 250 | text normalisation validations (stage 2) | 70 s |
| assets/pipelines/modalities/text/main.rs:4013 | 800 | text normalisation validations (stage 2) | 125 s |
| assets/pipelines/modalities/text/main.rs:4307 | 300 | text normalisation validations (stage 2) | 75 s |
| assets/pipelines/modalities/text/main.rs:4345 | 500 | text normalisation validations (stage 2) | 95 s |
| assets/pipelines/modalities/text/main.rs:4433 | 200 | text normalisation validations (stage 2) | 65 s |
| assets/pipelines/modalities/text/main.rs:5580 | 2000 | text normalisation validations (stage 2) | 245 s |
| assets/pipelines/modalities/text/main.rs:5843 | 800 | text normalisation validations (stage 2) | 125 s |
| assets/pipelines/modalities/text/main.rs:6289 | 900 | text normalisation validations (stage 2) | 135 s |
| assets/pipelines/modalities/text/main.rs:6813 | 100 | text normalisation validations (stage 2) | 55 s |
| assets/pipelines/general/zero_shot_simulation/main.rs:135 | 1500 | zero-shot simulation pipeline | 195 s |

Observations: (1) these are **hints** — the prompt pipeline ignores them for the API body unless `OZONE_API_FORCE_MAX_TOKENS=1` (`effective_api_cap`, prompt/main.rs:749), so they are not response caps on the API path; (2) they ARE used by `request_timeout()` (new), by the Anthropic path (`unwrap_or(4096)`, :637 — a REAL default cap), and by two local paths (`unwrap_or(1024)` :1073, `512` :1273); local llama/BitNet `-n` already derives from the window (`context_length/2`, :962/:1267/:1345); (3) reasoning models spend completion tokens on reasoning, so a hint of 20–300 says nothing about how long the call can legitimately take.

**Recommendation on `request_timeout()`:** do NOT keep deriving it from `max_tokens`. A timeout bounds *waiting on a provider*, which depends on prompt size, provider load and reasoning, not on the answer-length hint. Replace with `timeout = clamp(60 s + prompt_tokens/200 s, 90 s, 600 s)` (floor 90 s, because a 55 s timeout on `max_tokens:100` calls is below observed free-pool latencies), keep `OZONE_PROMPT_TIMEOUT_SECS` as an override, and let the fallback chain (not the timeout) provide resilience. Keep the doctrine that `max_tokens` never truncates an answer.

## 4. FIX LIST (class A/B only, recommended order; each is mechanical)

1. **[S1 · A · R-CAP] mcp/gates** — `src/orchestrator/tool_routing.rs (removed 2026-11-04): query[..200], MAX_RESULT_CHARS 2000, MAX_BLOCK_CHARS 4000, MAX_TOOLS_PER_STEP 2, `.take(12)` words` — tool routing caps  
   Fix: FIXED (whole step text, window-fit by caller, relevance group). Add a unit test that a 10 000-char step text reaches the tool whole.
2. **[S1 · A · R-CAP (+ crash)] orchestrator** — `src/orchestrator/mod.rs:2215-2230 `MAX_FILE_ANALYSIS_BYTES=200_000` + `content.truncate(..)`` — attached file cut at 200 KB before analysis; `String::truncate` PANICS if byte 200000 is not a char boundary  
   Fix: Remove the cut: analyse in windows/chunks to completion (batch-complete) or fail the file loudly; at minimum truncate on a char boundary and record `file_truncated` in the run record.
3. **[S1 · B · R-CAP R-SEARCH-FLOOR] orchestrator** — `src/orchestrator/mod.rs:2524 `state.keywords.iter().take(20)` → `search_by_keywords(.., Some("Methodology"))`` — methodology GATHER (the heart of Stage 3) searches only the first 20 request keywords  
   Fix: Search with ALL keywords (batch the store query), or rank keywords and loop until the set is covered; record coverage.
4. **[S1 · A · R-CAP] orchestrator** — `src/orchestrator/amt.rs:2598/2608 `rule_limit/heuristic_limit` (load_methodology_rules_text uses 5/3)` — `load_methodology_rules_text` limits to 5 rules + 3 heuristics; `_full` has no cap  
   Fix: Make every caller use `_full` + render_within_window; delete the limited variant (grep callers).
5. **[S1 · A · R-EMPTY] orchestrator** — `src/orchestrator/mod.rs:5440 `parse_json_object` → `unwrap_or_else(|_| json!({}))`` — unparseable model JSON becomes `{}`  
   Fix: Only used by its own test (dead). DELETE it (and its test) so it cannot be reused; `parse_json_object_strict` is the live path.
6. **[S1 · A · R-FAB R-EMPTY] orchestrator** — `src/consciousness/store.rs:1283-1290 `migrate_hook_gate_decisions` (`ethical_score … unwrap_or(1.0)`, `decision … unwrap_or("Proceed")`)` — migration of legacy gate records invents a Proceed decision with ethical score 1.0 when fields are missing  
   Fix: Skip/mark `unverified_migrated` instead of fabricating an approval; never default a gate decision to Proceed.
7. **[S1 · A · R-CAP] pipelines/prompt** — `assets/pipelines/general/prompt/main.rs:637 `"max_tokens": input.max_tokens.unwrap_or(4096)` (Anthropic path)` — Anthropic requires max_tokens; 4096 default is a hidden response cap  
   Fix: Default to the model's real maximum output (from the model profile) and document; when `max_tokens` is only a hint (doctrine), pass the maximum, not 4096.
8. **[S1 · B · R-CAP R-SEARCH-FLOOR] pipelines/context_aggregation** — `assets/pipelines/general/context_aggregation/main.rs:238 `MAX_TRAVERSAL_SEEDS=4`, :509 `MAX_RESOLVED_CONTAINERS=8`, `TRAVERSAL_MAX_DEPTH=2`, `TRAVERSAL_MAX_RESULTS=10`` — the old fixed-count context path still runs beside span-out  
   Fix: Delete the fixed counts: seed/resolve by score within the token budget (span_out_context already supplies the window-filled context); keep the old path only as a fallback marked as such.
9. **[S1 · A · R-CAP] pipelines/context_aggregation** — `assets/pipelines/general/context_aggregation/main.rs:618 `MAX_CHARS=1600` (coordination layer, byte-based `layer.len()`), :595 body `take(240)`` — coordination context cut to 1600 bytes / each note 240 chars  
   Fix: Fit to a token budget (share of the window); truncate on char boundaries.
10. **[S1 · A · R-CAP R-SEARCH-FLOOR] modalities** — `assets/pipelines/modalities/{3D:1541,BCI,CAD,IMU,biology,control,depth,electromagnetic,geospatial,haptic,hyperspectral,kinematics,network,proteomics,radar,sonar,sound,thermal}/main.rs `nodes.iter().take(30)` + `n.content.chars().take(80)` in `infer_semantic_relationships_validated`` — relationship inference prompt sees only the first 30 nodes, 80 chars each  
   Fix: Chunk the node list through the prompt to completion (batch) and send whole node content (window-fit) — otherwise cross-node relations beyond node 30 are never found. (The shared `semantic_relations` helper is the right place.)
11. **[S1 · A · R-CAP] tools** — `tools/browser-mcp/server.mjs:362 `String(input.text).slice(0, 5000)` (Input.insertText)` — typed text silently cut at 5000 chars  
   Fix: Type the whole text (chunked insertText) or error when over a stated limit.
12. **[S1 · A · R-CAP] tools** — `tools/terminal-mcp/server.mjs:148-157 `cap = 100 KiB` (`stdout.slice(0,cap)`)` — command output cut with no flag  
   Fix: Add `stdout_truncated:true` + total bytes; offer paging/`tail`.
13. **[S2 · A · R-CAP R-GATE] mcp/gates** — `tools/{web,dna,connectome,security,re,terminal,document,browser,browser-farm,p2p-bridge,threed}-mcp/server.mjs (`size > 1 * 1024 * 1024 → req.destroy()`; cerebrix/visual/code 20 MiB)` — oversized request body: socket destroyed, no reply  
   Fix: Return HTTP 413 + JSON `{success:false,error:'request body over N bytes'}`; make N env-tunable; the host then reports an honest error instead of 'tool unreachable'. (Any gate-bearing input larger than the limit is silently never gated downstream.)
14. **[S2 · A · R-CAP] mcp/gates** — `src/network/mod.rs:1212-1214 `DIALOGUE_MAX_BODY_CHARS=4000`` — `turn.body = turn.body.chars().take(4000).collect()` — peer dialogue silently truncated  
   Fix: Reject the turn with an explicit error (never relay a silently cut body) or size by the receiving model's window; record the trim.
15. **[S2 · A/B · R-CAP R-SEARCH-FLOOR] orchestrator** — `src/orchestrator/stages.rs:733 `.take(15)` (blueprint reuse search), :603 `.take(8)` + :617 `.take(3)` + :639 `.take(8)` (prior-design context: 8 kw, 3 blueprints, 8 step descriptions each), :1304 `.take(8)` (branch keywords), :1680 `.take(8)` (simulation related containers)` — first-N keyword/blueprint/step floors feeding reuse + prior design + simulation context  
   Fix: Use all keywords; fit prior-design text with render_within_window instead of 3×8; keep the 0.95 Jaccard reuse test on the FULL keyword set.
16. **[S2 · A · R-CAP] orchestrator** — `src/orchestrator/awaiting.rs:150,151,168,177,187 (subject 400, reason 800, resolution 800)` — awaiting reason/resolution text cut — this is what a human reviews  
   Fix: Store whole; truncate only at display time in the UI.
17. **[S2 · A/B · R-CAP R-SEARCH-FLOOR] orchestrator** — `src/orchestrator/graphs.rs:480 `.take(15)` keywords, :487 `.take(8)` topics (graph-creation prompt), :842 `.take(12)` + :849 `.take(6)` (prompt-term seeds)` — graph-creation prompt and seed discovery use first-N  
   Fix: Pass all keywords/topics, window-fit; batch seed search.
18. **[S2 · A · R-FAB] orchestrator** — `src/orchestrator/{mod.rs:2276,graphs.rs:162} `confidence … unwrap_or(0.5)` (math ParseExpression), src/zsei/hooks.rs:797,894,953 edge `confidence … unwrap_or(0.5)`` — missing model confidence becomes a fabricated 0.5 on graph edges/parse results  
   Fix: Store `None`/absent (or `null`), never 0.5; consumers treat absent as unknown (span-out already defaults edge weight, not the stored value).
19. **[S2 · A · R-CAP] orchestrator** — `src/context_needs.rs:179 `raw.chars().take(400)`` — context-needs extract cut at 400 chars  
   Fix: Window-fit instead of a fixed 400.
20. **[S2 · A · R-CAP] orchestrator** — `src/zsei/hooks.rs:605 `graph.nodes.iter().take(50)` ("Limit for prompt size")` — graph summary prompt shows only 50 nodes  
   Fix: Window-fit the node summary (or use the new fan-out digest).
21. **[S2 · A · R-CAP] pipelines/browser_navigation** — `assets/pipelines/general/browser_navigation/main.rs:267-269` — `innerText.slice(0,100000)`, links `slice(0,100)`, images `slice(0,50)`  
   Fix: Return everything with explicit paging/`truncated:true` + counts.
22. **[S2 · A · R-CAP] modalities** — `assets/pipelines/modalities/uidocument/src/html.rs:85-86 `MAX_TEXT=4000` text-node cut (silent), a11y.rs:14,63,196,245 `NAME_LIMIT=300` (silent), a11y_json.rs MAX_TEXT, android.rs MAX_TEXT 4000` — per-node text silently cut (graph-level guards DO report in `notes`)  
   Fix: Record `text_truncated` per node/in notes (same mechanism as the input-size note at html.rs:153) or keep the full text.
23. **[S2 · A · R-CAP] tools** — `tools/browser-mcp/server.mjs:243 page `text … slice(0,3500)`, :234 `.slice(0,150)` elements, :239 value 60, :242 labels 80` — perception snapshot cut (3.5 KB text, 150 elements)  
   Fix: Return `truncated:true` + counts and support paging (`snapshot {offset}`), or budget by window.
24. **[S2 · A · R-CAP] tools** — `tools/git-mcp/server.mjs:62 `return out.slice(0, 500)` (file_diff)` — diff lines capped at 500 silently  
   Fix: Return all lines or `truncated:true` + total.
25. **[S3 · B · R-CAP R-SEARCH-FLOOR] orchestrator** — `src/orchestrator/amt.rs:3258 `.take(12)`, :3357 `.take(8)`, :3375 `.take(3)` (related blueprints), :3448 `.take(6)`, :3468 `.take(12)` (hop seeds)` — AMT enrichment search/hops capped by count  
   Fix: Replace counts with score thresholds + window budget (as span-out does); record the cutoff.
26. **[S3 · B · R-SEARCH-FLOOR] orchestrator** — `src/orchestrator/amt.rs:2694-2739 `rank_methodologies(max_selected)`` — selection policy bounds how many methodologies earn branch-discovery lanes (`usize::MAX` = all)  
   Fix: Default to ALL (documented switch exists); if a bound stays, record uncovered methodologies (already logged as UNCOVERED lanes).
27. **[S3 · A/C · R-CAP] orchestrator** — `src/orchestrator/amt_candidates.rs:122 `detail.take(200)`, actors.rs:576 `detail.take(600)` (coordination finding body), actors.rs:313 `take(80)`, actors.rs:822 `take(120)`, amt_loop.rs:541, instrumented.rs:91 `take(300)`, response.rs:275 `take(60)` probe` — candidate/finding detail + log previews  
   Fix: actors.rs:576 puts a 600-char cut INTO a stored coordination note (A-lite): store whole. The rest are log/identifier previews (C).
28. **[S3 · A · R-FAB] orchestrator** — `src/consciousness/store.rs:1270 (`arousal … unwrap_or(0.3)`), :1339 (`relevance_score … unwrap_or(0.8)`)` — defaults for missing emotional/relevance values  
   Fix: Use Option/absent; the dashboard shows 'unknown'.
29. **[S3 · B · R-CAP R-SEARCH-FLOOR] orchestrator** — `src/zsei/query.rs:654 `direct.iter().take(12)` + `neighbors_injected >= 12`` — traversal neighbours injected: 12 seeds / 12 neighbours  
   Fix: Budget by tokens/score (span-out) instead of counts.
30. **[S3 · A · R-CAP] pipelines/prompt** — `assets/pipelines/general/prompt/main.rs:1073 `input.max_tokens.unwrap_or(1024)` (local ONNX/py path), :1273 `.unwrap_or(512)`` — local-path defaults  
   Fix: Derive from the loaded model window like -n (half the window).
31. **[S3 · B · R-CAP (indirect)] pipelines/prompt** — `assets/pipelines/general/prompt/main.rs `request_timeout()` (new)` — HTTP timeout derived from `max_tokens` (45 s + 1 s/10 tokens, cap 600 s)  
   Fix: See the max_tokens table: do NOT keep deriving from a caller hint; derive from prompt size + a floor of 90 s and the provider's observed latency.
32. **[S3 · D/B · R-CAP] pipelines/context_aggregation** — `assets/pipelines/general/context_aggregation/main.rs:325 `truncate_to_budget`, :720 `keyword_list.truncate(12)` (+ `>= 12` loop)` — token-budget trim; keyword list capped at 12  
   Fix: truncate_to_budget is budgeted (D). The 12-keyword cap is a B: use all keywords.
33. **[S3 · B · R-CAP R-SEARCH-FLOOR] modalities** — `assets/pipelines/modalities/*/main.rs (3D, BCI, CAD, IMU, audio, biology, chemistry, control, depth, dna, eeg, electromagnetic, geospatial, haptic, hyperspectral, image, kinematics, network, proteomics, radar, sonar, sound, thermal, video: `k.truncate(12)`; text: `keywords.truncate(20)` :5029,:5058)` — container keyword set capped at 12 (20 for text) — keywords drive `search_by_keywords` discovery  
   Fix: Keep all distinct keywords (dedup + rank, no count cut); at most bound by the container size budget.
34. **[S3 · B · R-CAP (intent)] modalities** — `assets/pipelines/modalities/text/main.rs:2758,3213,3505,3767,4013,4307,4345,4433,5580,5843,6289,6813; math:1951 `"max_tokens": 100…2000`` — fixed `max_tokens` literals at ~13 call sites  
   Fix: See table; they are hints (API cap ignored) but feed `request_timeout()` and local paths.
35. **[S3 · B · R-CAP R-SEARCH-FLOOR] ui** — `ui/src/views/search/UnifiedSearch.tsx:60 `ids.slice(0,60)`, :102/:119 `hits.slice(0,30)`` — search coverage capped (60 containers resolved, 30 results) with no 'more' affordance  
   Fix: Page ('load more') and show total counts.
36. **[S3 · B · R-CAP] ui** — `ui/src/graphkit/layout3d.ts:83-85 `MAX_GRAPH_NODES=2400, MAX_CONTAINER_NODES=300, MAX_FILE_NODES=300` + :192,:289,:329 slices` — 3D layout shows a subset of the graph  
   Fix: Level-of-detail / on-demand expansion with a visible 'N of M shown' (verify one exists); the fan-out lens must not hide the entries.
37. **[S4 · C/B · R-CAP] ui** — `ui/src/panels/enginesRegistry.tsx:89 `nodes.slice(0,200)`, views/hierarchy/AmtTreeView.tsx:421 `chunkMatches.slice(0,20)`, graphkit/GraphNativeWorkspace.tsx:297,490,564 (14/8/14), graphkit/lens.tsx:57 `nodes.slice(0,50)`, files/FileReferenceViewer.tsx:193 `relations … slice(0,20)`` — list renderers show first N  
   Fix: Display bounds; add 'show more' where data is hidden (FileReferenceViewer:193 resolves only 20 relation targets — B).

(Already fixed this wave and recorded for completeness: `mcp.rs` `check_call_gate` whole-text; `tool_routing.rs` caps; Stage 3 first-8 floor.)

## 5. Not verified / out of scope (stated plainly)
- **Stage-0 text normalisation before gating:** not traced end-to-end; confirm the jurisdiction intake sees the raw prompt, not a normalised/trimmed copy.
- **Context-window derivation:** every `render_within_window`/`trim_at_paragraph` caller was not individually checked for a literal budget; grep for literal second/third arguments.
- **`tools/bridges` (TypeScript dist, 67 tools)** and the Python workers under `tools/*/` (cerebrix, connectome, dna, mesh, visual, code-graph, re-mcp) were scanned only for obvious `[:N]`/`slice` patterns in the `.mjs` front ends — their Python internals still need a pass.
- `ui/src/**` display slices were classified by name/context, not behavior; items marked C could hide data without a 'more' affordance.
- Counts are of GROUPED rows (same pattern in many files = one row), not raw regex hits; raw hit counts: 110 `chars().take`, 74 `MAX_*`, 64 `truncate`, 141 `slice(0,N)`, 32 `max_tokens`.


---

## 6. FIX STATUS (2026-11-04, edits applied under "no build, no test" — UNCOMPILED / UNTESTED)

**Operator doctrine added after this audit was written: NO SELF-REPORTED CONFIDENCE** (docs/CONTRACTS.md §5.1/§5a; memory `feedback_no_self_reported_confidence`). Where this audit's fix #18 said "store `None`/absent instead of 0.5", the stronger ruling applies: the field is REMOVED (confidence/importance are no longer requested from models, thresholded, or used as edge weights); relationships are present/absent, validated by the yes/no strength-N loop. Applied in `zsei/hooks.rs`, span-out, fan-out, meta-loop prompt; math-parse absence = 0.0. The stored `Relation.confidence` and the decision-gate `confidence` remain to retire (EDIT_MANIFEST item 23).

| # | Status | # | Status | # | Status |
|---|---|---|---|---|---|
| 1 | ✅ tool routing (whole step text; window-fit; relevance group) | 14 | ✅ dialogue turn CHUNKED into numbered parts and sent whole (never cut, never refused for size) | 27 | ✅ actors.rs:576, amt_candidates (rest are C) |
| 2 | ✅ files read and analysed WHOLE — no size ceiling, no refusal (chunking is downstream) | 15 | ✅ stages.rs keyword/blueprint/step floors removed | 28 | ✅ legacy emotion/perception migrations skip, no invented 0.3/0.8 |
| 3 | ✅ all request keywords search methodologies | 16 | ✅ awaiting text whole | 29 | ✅ methodology hop expansion: every seed/neighbour |
| 4 | ✅ `load_methodology_rules_text` = full | 17 | ✅ graphs.rs first-N removed | 30 | ✅ local ONNX/bitnet defaults = half the window |
| 5 | ✅ `parse_json_object` deleted | 18 | ✅ superseded by confidence removal | 31 | ✅ `request_timeout` from prompt size, floor 90 s |
| 6 | ✅ legacy gate migration skips (no invented Proceed/1.0) | 19 | ✅ context_needs whole | 32 | ✅ 12-keyword cap removed |
| 7 | ✅ Anthropic max_tokens = model family ceiling (override env) | 20 | ✅ hooks node summary: every node, whole | 33 | ✅ 24 modality + text keyword truncates removed |
| 8 | ✅ context_aggregation seed/container counts removed (token-budget stop) | 21 | ✅ browser_navigation whole page/links/images | 34 | ◻ text/math `max_tokens` literals = hints; documented, not removed |
| 9 | ✅ coordination layer budget-based; note bodies whole | 22 | ✅ uidocument per-node text/names whole | 35 | ✅ UnifiedSearch resolves every id, lists every hit |
| 10 | ✅ 16 modalities: overlapping windows cover EVERY node, content whole | 23 | ✅ browser snapshot reports `truncated` totals | 36 | ✅ verified: `WorldTruncation` already reports N of M |
| 11 | ✅ typed text whole (chunked insertText) | 24 | ✅ git diff all lines | 37 | ◻ display slices (class C) — add "show more" later |
| 12 | ✅ terminal reports `*_truncated` + total bytes | 25 | ✅ amt.rs enrichment counts removed | | |
| 13 | ✅ 14 MCP servers: transport guard raised to 512 MiB (`OZONE_MCP_MAX_BODY_BYTES`); beyond it an HTTP 413 JSON reply (host-memory guard, not a model limit) — never a silent destroy | 26 | ✅ already `usize::MAX` at the call site | | |

Still to verify after the first compile: `semantic_relations::windows` coverage test, the 16 modality wrapper signatures, `context_aggregation` call sites (`resolve_containers(…, Option<u32>)`), `build_coordination_layer(…, token_budget)`.
