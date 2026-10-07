# Context flow coverage audit, 2026-10-06

Read-only audit by fork CTX-AUDIT. No source, config or store was edited. No build, test, host call or LLM call. Line numbers are as read during the audit; the INTENT, CAPS, WINDOW and TEXT forks were editing `amt.rs`, `stages.rs`, `mod.rs` and the text pipeline at the same time, so line numbers may have shifted.

## Summary

- **Model-call sites (Part A, table A1, 28 rows):** 5 log a full ContextRecord (window and prompt size). 6 log partially (walk fallback only, decision review previews, or zero-shot preview only). 16 log nothing. Row 28 is covered by row 5.
- **Stage handoffs (Part B):** 0 of 10 handoffs write a record. They are covered only by tracing lines, in-memory state and, for some, ZSEI graph writes.
- **Join key (Part B):** none. There is no run or request id. `task_id` is optional and unset for direct `/orchestrate` runs. The gate input sends `"task_id": 0` as a literal.
- **Critical finding:** the consciousness decision gate (pipeline 39) does not read the plan it is sent. See section D.

## A. Model-call sites

Coverage key: **Full** = ContextRecord with window and prompt size. **Walk** = ContextRecord only for fallback candidates. **Z** = `zero_shot_calls.jsonl` only (no window, prompt size only as a 200-char preview). **None** = no record.

### A1. Call sites

| # | file:line | call_site / label | pipeline | path | Coverage | Window source | Prompt size recorded |
|---|---|---|---|---|---|---|---|
| 1 | orchestrator/i_loop.rs:146 | i_loop_reflection | 9 | capture_loop_model_call | Full | registry (served model) | yes |
| 2 | orchestrator/amt_loop.rs:622 | amt_reexpansion | 9 | capture_loop_model_call | Full | registry | yes |
| 3 | orchestrator/meta_loop.rs:266 | meta_loop_draft | 9 | capture_loop_model_call | Full | registry | yes |
| 4 | consciousness/assistant.rs:387 | assistant_check_up | 9 | capture_loop_model_call | Full | registry | yes |
| 5 | orchestrator/amt.rs:2646, 2650 | lane labels passed by caller | 9 | capture_loop_model_call, once per joined lane result | Full (per lane, not per part) | registry | yes |
| 6 | orchestrator/mod.rs:3255 | fallback_walk:pipeline-N | any | walk candidate attempts | Walk | registry or configured | yes |
| 7 | orchestrator/decision_review.rs:179 | decision review, via walk | 9 | walk, plus its own decision_review.jsonl | Walk + D (previews only) | registry or configured | previews only in D |
| 8 | orchestrator/mod.rs:2401 | every metered_execute primary call | 9 | executor.execute, no walk | Z | none | preview only |
| 9 | orchestrator/mod.rs:2745 / 2812 | metered_execute_resilient labels: blueprint_assignment, zero_shot_simulation, web_search_decompose, context_compaction, methodology_compliance_check, methodology_domain_id, methodology_synthesis, response_graph_render (labels seen in the file) | 9 | resilient inner, primary attempt | Z | none | preview only |
| 10 | orchestrator/stages.rs:2244, 2263 | stage 10 step primary | per step (often 9) | direct executor | None | none | no |
| 11 | orchestrator/stages.rs:2284 | stage 10 step fallback | 9 | try_fallback_chain, walk | Walk | registry or configured | yes |
| 12 | orchestrator/stages.rs:2151 | sub-step execute | coerced to 9 | direct executor | None | none | no |
| 13 | orchestrator/stages.rs:1525 | consciousness gate | 39 | direct executor | None | none | no |
| 14 | orchestrator/jurisdiction.rs:522 | jurisdiction review judgment | 39 | direct executor | None | none | no |
| 15 | orchestrator/mod.rs:3375, 3382 | strength-N confirmation votes | 9 | direct executor, no state, no walk | None | none | no |
| 16 | orchestrator/mod.rs:3680 | ILoop | 44 | direct executor, `if let Ok` (errors dropped) | None | none | no |
| 17 | orchestrator/mod.rs:1712 | voice | 10 | direct executor | None | none | no |
| 18 | orchestrator/mod.rs:2091 | reconstruct | 100 | direct executor, `?` | None | none | no |
| 19 | orchestrator/mod.rs:2206 | methodology create | 12 | direct executor | None | none | no |
| 20 | orchestrator/stages.rs:1852, 1894 | web search | 56 | direct executor | None | none | no |
| 21 | orchestrator/jurisdiction_search.rs:118 | web search | 56 | direct executor | None | none | no |
| 22 | orchestrator/stages.rs:2555, 2567, 2579 | experience, relationship, emotion | 41, 47, 43 | direct executor, result discarded | None | none | no |
| 23 | orchestrator/response.rs:392 | voice output | 46 | direct executor, `if let Ok` | None | none | no |
| 24 | orchestrator/response.rs:428 | dashboard | 54 | direct executor, result discarded | None | none | no |
| 25 | orchestrator/response.rs:441 | task recommendation | 23 | direct executor, result discarded | None | none | no |
| 26 | orchestrator/graphs.rs:132, 173, 243, 265, 285, 356, 382, 411 | graph builders, text and link pipelines | various (text pipeline calls a model) | direct executor, `let _` or `if let Ok` | None | none | no |
| 27 | orchestrator/lane_split.rs:147, 165 | lane parts | 9 | direct executor, per part | None per part (only the joined lane result, row 5) | none | no |
| 28 | orchestrator/amt.rs:2622 | AMT lane spawn | 9 | direct executor inside spawn | see row 5 | see row 5 | see row 5 |

**Coverage count (28 rows):** Full 5 (rows 1 to 5). Partial 6 (rows 6, 7, 8, 9, 11 and 27). None 16 (rows 10 and 12 to 26). Row 28 is the same lane spawn as row 5. Row 9 bundles several call_site labels; each label gets the Z record and the walk record on fallback.

Ripple paths (`amt_loop.rs` spawn_graph_ripple_sync, `actors.rs`, `file_beacon.rs`, `task/mod.rs`): **no direct executor call** found by grep. They are not model sites.

Not verified by reading the crate: whether pipelines 41, 43, 47, 100, 12, 44, 46 and 54 call a model. Pipelines 9, 56 and the text pipeline do (grep found model-call functions in those crates). Pipeline 23 (`general/task_recommendation`) has no model reference.

### A2. Record writers

| writer | sink | records |
|---|---|---|
| `context_budget::record_call` | `{OZONE_ZSEI_DATA_DIR or zsei_data}/capture/context_records.jsonl` | ContextRecord (walk and loop) |
| `capture_zero_shot_call` (mod.rs ~2880) | `{self.data_dir}/model_calls/zero_shot_calls.jsonl` | stage primary calls |
| `DecisionReviewExecutor::capture` | `{data_dir}/model_calls/decision_review.jsonl` | decision review |
| `zsei writes audit` | `zsei_writes.jsonl` | ZSEI writes (not model calls) |

Two data-dir sources: the context sink reads the env var `OZONE_ZSEI_DATA_DIR` (default `zsei_data`). The zero-shot sink reads `config.general.data_dir` (lib.rs:1320). The current config sets both to `zsei_data`, so they agree now. They can diverge if either is changed.

## B. Stage-to-stage handoffs

| # | handoff | file:line | object handed | logged today | run or request id on it |
|---|---|---|---|---|---|
| 1 | request to state | mod.rs (OrchestrationRequest, line 115) | prompt, user_id, project_id, workspace_id | none | no run id; project_id only |
| 2 | jurisdiction result | jurisdiction.rs:343 | JurisdictionGateResult | tracing; the pipeline-39 call (row 14) is unlogged | no |
| 3 | cleaned prompt | mod.rs:2093 | cleaned_prompt | none (before and after not recorded) | no |
| 4 | intents and AMT | amt.rs:143 | state.amt, ZSEI AMT containers | ZSEI writes audit; amt_container_id on 27 of 407 zero-shot rows | amt_container_id, partial |
| 5 | branches to blueprint | stages.rs (blueprint prompt ~531-620) | branch names as "name: N children" | tracing only | no |
| 6 | blueprint | stages.rs:397, 1060 | blueprint_id, steps (stages.rs:719) | blueprint_id is null in all 407 zero-shot rows | blueprint_id, in practice none |
| 7 | step outputs | stages.rs stage 10 (~2166-2290) | step output to all_outputs | tracing per step; no record of step input composition | no |
| 8 | simulation | stages.rs:1468 | SimulationOutcome | zero-shot row (call_site zero_shot_simulation) only | no |
| 9 | gate decision | stages.rs:1620 | GateResult | tracing; input dropped (section D) | no; the gate sends `task_id: 0` |
| 10 | delivery | stages.rs stage 13 (~123) | final response | tracing; thinking_log | no |

The thinking log is persisted through the task record (`task/mod.rs:2084`). Direct `/orchestrate` runs with no task were **NOT VERIFIED** to persist it.

Handoff count: **0 of 10** write a record that carries a run id. 1 carries a partial id (row 4, AMT container, present on 27 of 407 zero-shot rows). 9 carry no usable id (row 6's blueprint_id is null in every record).

## C. Existing record shapes

Counts are from the captured files in `target/release/zsei_data` (read-only).

**ContextRecord** (`capture/context_records.jsonl`, 36 rows): `ts_ms`, `call_site`, `model`, `window_tokens`, `want_output_tokens`, `prompt_tokens`, `usable`, plus `trims` in the code. `window_source` is in the code now (WINDOW fork) but is **not** in the captured rows, which predate it. There is no project, blueprint, AMT, user or run field.

**zero_shot_calls.jsonl** (407 rows): `ts`, `call_site`, `model_used`, `tokens_used`, `retry_count`, `used_fallback`, `success`, `response_preview` (500 chars), `prompt_preview` (200 chars), `amt_container_id` (non-null in 27 rows), `blueprint_id` (null in all 407 rows), `project_id` (non-null in 10 rows). No window, no prompt size.

**decision_review.jsonl** (21 rows): `ts`, `model_used`, `tokens_used`, `decision`, `confidence`, `reasoning_preview`, `raw_response_preview` (15 of 21), `task_summary_preview`. No correlation id.

**What a reader can join today:**
- ContextRecord to zero-shot: by `call_site` and timestamp only. Not reliable.
- zero-shot to project or AMT: possible for the 10 to 27 rows where the fields are set.
- Any stream to a run: impossible. No run id exists.
- prompt size across streams: the context sink counts `prompt_tokens` for the walk including the system prompt (`(p.len() + sys.len()) / 4 + 1`) and for loops from the prompt string (`text.len() / 4 + 1`). Both are byte-based. Zero-shot has no prompt size.

## D. Critical finding: the gate and jurisdiction judgments do not read their input

Verified by reading `assets/pipelines/consciousness/decision_gate/main.rs`:

- The gate's `Evaluate` input has `context: Option<...>` and `execute` matches it with `context: _`, so it is ignored (line 397).
- `run_ethical_assessment(task_summary, config)` does not use `task_summary`. Its principles are constants: 0.95, 0.90, 0.92 with fixed reasoning strings (lines 265 to 298).
- `run_emotional_response`, `run_experience_retrieval` and `run_identity_alignment` take `_task_summary` and ignore it (lines 300 to 335).
- `make_decision` receives only those constant-derived structs and the config.

The host sends the full review (request, AMT, blueprint steps, jurisdiction, methodology rules, simulation predictions) as `task_summary` (`stages.rs:1574-1590`). The jurisdiction review does the same (`jurisdiction.rs:500-523`). Neither pipeline reads it.

Consequence: the stage-8 gate decision and the jurisdiction review decision do not depend on the plan. Any Proceed decision observed from pipeline 39 reflects these constants. ZCode's report of "gate Proceed 80%" comes from this path. This is not a logging gap; it is a context gap that the logging would expose.

## E. Prioritized gaps

1. **Pipeline 39 discards its input** (section D). Stage 8 and jurisdiction decisions do not read the plan. Needs an operator decision on the intended gate, then a fix inside the decision_gate crate.
2. **No run id.** Add an orchestration id minted per request, carried in `OrchestrationState`, and written on every record: ContextRecord, zero-shot, decision review, ZSEI write audit, thinking log. Without it nothing else can be joined.
3. **Primary attempts have no ContextRecord.** `metered_execute` (mod.rs:2401) is the path for every stage's first pipeline-9 call and writes only Z. Route it through the same record hook as the walk, with window and prompt size.
4. **Non-9 model-calling pipelines are unrecorded** (rows 13 to 28). The text pipeline, web search (56), voice (10, 46) and methodology (12, 100) need a common record hook.
5. **Strength-N votes are unrecorded** (row 15). They call pipeline 9 directly with no state and no walk.
6. **Silent error drops:** `mod.rs:3680` (`if let Ok`, pipeline 44), `response.rs:392` (46), `graphs.rs` `let _`. Each should record the failure.
7. **Zero-shot correlation is mostly null.** `amt_container_id` is set on 27 of 407 rows, `project_id` on 10, and `blueprint_id` on none. Populate them at each call site, not only where state happens to hold them.
8. **Gate input has `task_id: 0`.** Pass the run id instead.
9. **Stage handoffs (section B) need records.** At minimum: stage, handed object id, count, and run id, for every handoff.
10. **Two data-dir sources** (section A2). Use one.
11. **Estimator is byte-based** (`len / 4`). Non-ASCII text is overcounted. Use the same estimator everywhere and state it in the record.

## Not verified

- Whether pipelines 41, 43, 47, 100, 12, 44, 46, 54 and the graph-builder pipelines call a model (section A1 note).
- Whether the thinking log persists for direct `/orchestrate` runs with no task.
- Whether the lane labels passed to `capture_loop_model_call` at amt.rs:2646 and 2650 are the ones listed in the zero-shot counts.
- The consciousness gate's stage-8 decision in the running binary (read from source only).
