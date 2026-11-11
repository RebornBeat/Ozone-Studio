# Switches default ON, tool publish privacy, tool-creation lifecycle (2026-11-11)

Edit only. Parse-checked with rustfmt (no error output). Nothing built, nothing tested, EU untouched.

## 1. Switches: all default ON in code

One reader, `config::switch_on(name)` (`src/config/mod.rs`, after `default_true`): unset, or any value except `off`/`0`/`false`/`no`, is ON. A host opts out only in its own environment or config.

| Switch | Before | Now | File |
|---|---|---|---|
| `OZONE_REQUIRE_SESSION` | on only `=1` | ON (3 readers: `entry_gate::require_identity`, `grpc/mod.rs` auth middleware, rerun identity comment) | `src/orchestrator/entry_gate.rs`, `src/grpc/mod.rs` |
| `OZONE_ZSEI_REQUIRE_SESSION` | on only `=1` | ON | `src/grpc/mod.rs` (ZSEI write refusal) |
| `OZONE_AUTO_EXECUTE_TASKS` | on only `=1` | ON | `src/lib.rs` (task executor) |
| `OZONE_PURGE_ORPHAN_GLOBALS` | on only `=1` | ON — idempotent, only tombstones empty torn-write shells | `src/zsei/storage.rs` |
| `OZONE_JURISDICTION_FORCE_SYNC` | on only `=1` | ON, made safe: a force-adopt happens ONCE per file per shipped hash (manifest marker `forced:<key>`), old copy kept as `.bak`; an operator edit made afterwards survives every later boot | `src/lib.rs` `sync_jurisdiction_file` |
| `OZONE_PROBE_FREE_MODELS` | on only `=1` | ON — probes every catalog free model once per boot and spends free-model quota (the operator's ruling; expect it in the first boot log) | `src/lib.rs` |

Test changed: `entry_gate::require_identity_only_blocks_when_strict_and_unvalidated` now asserts default = refuse, `off` = allow, `1` = refuse.

**Conflict left for the operator (not flipped, not silent):** these two are inverted-polarity measurement toggles; ON removes a capability.
- `OZONE_KEYWORD_IGNORE_EXPANSION` — ON would ignore every zero-shot expansion term in search (`keyword_program.rs`, ablation to measure what expansion adds).
- `OZONE_KEYWORD_STOPLIST` — ON would filter by a fixed stopword list; standing doctrine: stoplists only as a tested baseline, off by default (`request_terms.rs`).
Not switches: `OZONE_SEARCH_DEFAULT` (strategy name), `OZONE_MCP_DAILY_LIMIT` (0 = unlimited), tunables/paths.

Already on before this change: tool creation, refinement action/retire, offer review, keyword hits, settlement (EU holds it paused by its own drop-in), marketplace publish/accept, strict search (guardrail-audit commit d4b59fd).

Plan updated: `docs/BUILD_AND_TEST_PLAN_2026-11-10.md` line 92 (the "left as explicit opt-in" paragraph is replaced). Note for the first EU boot: `OZONE_REQUIRE_SESSION` and `AUTO_EXECUTE_TASKS` are already `=1` there; the new default-on ones that change EU behavior are the jurisdiction force-adopt (once per shipped version, `.bak` kept), the orphan purge, and the free-model probe.

## 2. Tool publish privacy

What leaves when a created tool is published (code: `tool_creation::PUBLISH_RULES`; methodology: `docs/evidence/methodology-from-prompts/drafts/30_tool-publish-privacy.json`, 6 rules, `requires` empty, every rule applies whenever gathered; listed in drafts `INDEX.json`):

- Leaves: the tool's own body only (name, description, capabilities, contracts, implementation) — `body_json`.
- Never leaves: `origin.json` (gap, request, task id, methodology id), the request, user content, operator data, secrets, local paths/identity, blueprints, jurisdiction content, ingest/bookkeeping nodes. (The existing code already kept origin out of `tool.json`; blueprints/ingest were already excluded at the announce chokepoint.)

New code-side check, in `stages::create_tool_for_capability` (after register, before `publish_created_tool`):
1. Deterministic scan `tool_creation::privacy_findings(body, implementation, request, data_dir)`: request text verbatim, absolute data-dir/`$HOME` path, credential shapes, literal assigned to a secret-named value. Real matching; no size limit.
2. If the scan is clean: ONE strength-N yes/no vote (`confirm_yes_no_batch`, prompt = rules + body + implementation). Publishing is the assured outward action, so every vote must say YES; one NO withholds. No confidence number.
3. Withheld: the tool stays created and registered locally; only the announcement is withheld; reason recorded as gap event `publish_withheld` (by `tool-publish-privacy`) and keyword event `tool_publish_withheld`. Published: unchanged.
4. Judgment recorded as call site label `tool_publish_privacy` (`record_judgment`), so it feeds the same hit/judgment log as every other vote.

Not addressed (needs a ruling): peers' imported methodologies get no equivalent placeholder/empty-rule check (guardrail audit gap, unchanged).

## 3. Where tool creation sits in the lifecycle (traced)

Walk order (`src/orchestrator/stages.rs`): stage 3 blueprint (+AMT built before) → **stage 6 capability closure** (~l.299–319: judge → methodology → tool, replan loop) → stage 4 simulation → **stage 7 post-simulation closure** (~l.337–360) → `stage_6_to_8_execute_steps` creates the TASK (`enqueue_task`, ~l.2925) then `materialize_work_order` (once) → step loop → stage 9 `work_order_walk_done` + `complete_task`.

So tool creation fires inside the OPEN WALK, after the AMT exists and before execution/completion — never after completion. But it fires BEFORE the task record exists, and its checklist was only ever a result of reading history afterwards. Defects found and fixed:

- **FIXED D1 (the real one):** `materialize_work_order` built gap tasks from `state.missing_capabilities`, but the closure REMOVES a gap from that list the moment it is judged not-needed / gets a methodology / gets a tool. So exactly the acted-on gaps got no `wo_capability` task: their `judge/methodology/tool/register/replan/verify` items never existed, `tool_created` events ticked nothing, and `work_order_walk_done` never verified them. Fix (`src/orchestrator/work_order.rs`): `gap_names(state)` = still-missing ∪ `closure_attempted` (control markers `__…` excluded), used by `materialize_work_order` and `work_order_walk_done`. `verify` now also fires for a gap closed by a created tool (previously only methodology-drafted/satisfied).
- Effect: created-tool work is now a child item (`tool`, then `register`) of a `wo_capability` task under the open walk task, ticked by the real `tool_created` / `tool_registered` events, in the work-order tree before the walk completes.

## 4. Open (not fixed — needs design / operator): expansion while the task is open

- **In-walk AMT expansion** (`on_step_complete`, `src/orchestrator/mod.rs` ~l.5260–5340): a step's alignment review can add `branch_captures`, with a comment that "new steps will be synthesized at the next iteration of execute_all_steps" — that function no longer exists; the execution loop's `step_queue` is built once from `state.blueprint_steps`. So a discovered branch produces no step, no re-run of the closure, and no work-order node. I added an event (`amt_branch_discovered_in_walk`, via `amt::record_intent_event`) so it is at least recorded. Making it real = after a step reports an expansion: re-run the capability closure for it, `stage_3_blueprint_assignment` replan with `state.replan`, extend `step_queue`, call `materialize_work_order` again (idempotent; grows under the walk), tick through the same events. Not done blind (uncompiled, touches the step loop).
- **Background AMT re-expansion** (`amt_loop.rs` ~l.418): registers a child task with `parent_task_id` = the walk (`find_walk_for_amt_container`) but the walk's own `complete_task` "does not wait for it". Under the operator's rule (completion = all steps/items complete, expansion while open is new items) the walk should not complete while an expansion child is open, and a completed walk stays final (expansion then starts a NEW task instance/graft). Needs: walk completion check for open `amt-loop` children, and a decision for failed expansions. Not done.

## 5. Other actions created during the phase — lifecycle position

| Action | Where | Relative to the task |
|---|---|---|
| methodology draft on a gap (`draft_methodology_for_capability`, pipeline 12 Create) | stage 6/7 closure | inside the open walk, before the task record; now has a `methodology` item via D1 fix |
| tool creation | stage 6/7 closure | same; `tool`+`register` items via D1 fix |
| gap fill by peer offer | removed (findings, not offers) | n/a |
| replan after creation | stage 6/7 loop + `work_order_after_replan` | open walk; `replan` item |
| refinement action (`act_on_refinement_candidates`, pipeline 12 Create/Update) | `meta_loop` background pass | OUTSIDE any walk/task (own pass); recorded in `methodology_refinement_actions.jsonl`, not in the work-order tree |
| AMT re-expansion | `amt_loop` background | child task of the walk but not gating it (§4) |
| marketplace accept | operator gate | outside; never part of a walk |

## 6. Unverified

Everything is parse-only. Unverified: `config::switch_on` callers compile; the forced-marker change in `sync_jurisdiction_file` (the existing test `sync_jurisdiction_file_upgrades_only_when_safe` forces once and should still pass — re-run); `create_tool_for_capability` borrow/move shape (`art.body_hash`/`body_path` moved only inside the publish branch; `state.keywords` read while `spec` is held — same pattern as before); `gap_names` effect on a real walk (rule gate and work-order tree have never run on the EU).
