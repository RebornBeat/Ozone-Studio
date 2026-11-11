# Guardrail audit + switch defaults — 2026-11-11 (read from code; NOTHING built or run)

Operator ruling (2026-11-11): every switch defaults ON in code; EU keeps all on; a local host turns things off only in its OWN config/env, never in code.

## 1. Switch inventory (grep of `env::var` / `[network]` / `[tasks]`) and what changed

| Switch | Where | Before | Now | Note |
|---|---|---|---|---|
| `OZONE_TOOL_CREATION` | `orchestrator/tool_creation.rs:35` | ON (off = `off`/`0`/`false`) | ON | unchanged |
| `OZONE_REFINEMENT_ACTION` | `meta_loop.rs:235` | ON | ON | unchanged |
| `OZONE_REFINEMENT_RETIRE` | `meta_loop.rs:408` | ON | ON | unchanged |
| `OZONE_KEYWORD_SETTLE` | `keyword_loops.rs:82` | ON (loop runs) | ON | EU is PAUSED by the operator's explicit drop-in `keyword-settle-off.conf` — a state, stays until the operator lifts it |
| `OZONE_KEYWORD_REQUIRE_VALIDATED` | `keyword_program.rs:218` | **OFF** | **ON** (`0`/`off`/`false` = off) | strict search, gated types only (`strict_for_type`). Order on EU: finish validation + namespace backfill before comparing search results |
| `marketplace_publish` / `marketplace_accept_offers` | `config/mod.rs` (`default_true`, `Default` impl) | **false** | **true** | local config can write `= false`. Gate unchanged: manifests only, ALLOWED kinds only, operator `download` decision, hash re-verify, blueprints never leave |
| `auto_broadcast_sync` (alias) | `config/mod.rs:626` | false | false | alias = "both on"; redundant now, kept for old configs |
| `OZONE_OFFER_REVIEW` | `offer_review.rs:194` | ON | ON | unchanged |
| `OZONE_KEYWORD_HITS` | `keyword_program.rs:256` | ON | ON | unchanged |
| `refinement_enabled` | `config/mod.rs:531` | ON | ON | unchanged |

Not flipped on purpose (not feature switches — destructive / one-shot / auth / measurement / baseline; the operator can override any): `OZONE_REQUIRE_SESSION` (auth posture; EU already sets 1), `OZONE_AUTO_EXECUTE_TASKS` (EU already sets 1), `OZONE_PURGE_ORPHAN_GLOBALS` (destructive purge), `OZONE_JURISDICTION_FORCE_SYNC` (overwrites operator edits), `OZONE_PROBE_FREE_MODELS` (spends quota), `OZONE_KEYWORD_IGNORE_EXPANSION` (ablation), `OZONE_KEYWORD_STOPLIST` (tested baseline, operator doctrine: off), `OZONE_SEARCH_DEFAULT` (strategy name), `OZONE_MCP_DAILY_LIMIT` (0 = unlimited; no quota doctrine). **Decision for the operator:** if "all" should include these, say which; I did not make a destructive purge or an auth gate default-on unasked.

Edited: `src/config/mod.rs` (default_true, Default impl, docs), `src/keyword_program.rs` (strict default ON), `src/lib.rs` (log text only), `docs/BUILD_AND_TEST_PLAN_2026-11-10.md` (§5.1 drop-in removed, §6 rewritten, §7 items 3/5/6, §7a). Possible knock-on on first compile/test: any test asserting the old defaults (none found by grep of `assert` near `publish_enabled`/`require_validated_keywords`).

## 2. Guardrails per action path (file:line, read from code)

| Action path | Guardrails running through it | Where | Missing / weak |
|---|---|---|---|
| Orchestrate walk entry | session identity (`OZONE_REQUIRE_SESSION`, EU on); jurisdiction soft gate (Stage 0) | `entry_gate.rs:151`, `stages.rs:224` → `jurisdiction.rs:348` | none new |
| Walk stages 2–7 | model calls metered/resilient; zero-shot every call (never dropped); capability closure progress-bounded (no count cap) | `stages.rs:230–360` | — |
| Resource guard | host-MEMORY floor `OZONE_MIN_AVAIL_MB` (1536) | `lib.rs:1437–1445` | applies ONLY to the queued-task executor, not to an interactive `/orchestrate` walk; adding one would be a refusal, not done — operator decision if wanted |
| Blueprint creation (draft) | one-pipeline-9 coercion for non-provider steps (non-9/56 → 9); provider steps need a named provider; `missing_capabilities` plain strings | `stages.rs:1832–1850`, `:64` | — |
| Blueprint review stages | rule gate (applicability + step fidelity), citation audit, coverage record, refinement | see §3 | — |
| Stage 8 consciousness | optional gate | `stages.rs:371` | — |
| Stage 8b jurisdiction FULL gate | judges the resolved plan before any step executes | `stages.rs:381` → `jurisdiction.rs:503` | — |
| Step execution (pipeline step) | task row first, per-step context scoped to the step's AMT branch; `StepGate` (jurisdiction Block re-check on step text) | `stages.rs:3384`, `tool_routing.rs:24–62,175` | gate only if installed (logs when not) |
| Provider/tool step | jurisdiction call gate on every in-process/HTTP call, metering, registry-only tools, model tool-choice only among the provider's real tools | `mcp.rs:141–211`, `stages.rs:3200`, `lib.rs:1341` | — |
| Stdio (incl. created) tools | host never spawns; invoke only returns "delegated" | `mcp.rs:651`, `tool_creation.rs:202` | — (no arbitrary code runs host-side) |
| methodology_create (pipeline 12) | placeholder/empty draft discarded; Update `remove_*` exact-match, versioned, never empties; Create needs real rule content | `meta_loop.rs:1256–1279`, pipeline 12 | old crate drops app-root content for ids 1000–1004 (known; rebuild) |
| Refinement action | strength-N vote, unanimity to split; verify new methodology before retiring; retire all-or-nothing + never-empty; evidence-gated (candidates only); kill switches | `meta_loop.rs:271–365,406–430` | — |
| Tool creation | validated slug (no path chars), collision check vs registry + created_tools, spec shape validated, `create_new` files (never overwrite), cleanup on failure, stdio + `auto_start=false`, judgment recorded, honest `skip` allowed, prompt forbids request content | `stages.rs:1156–1240`, `tool_creation.rs:47–195` | **no post-draft leak check**: privacy of the published body (tool.json incl. implementation) rests on the prompt only. A parameter-free check would be a model judgment, not added unasked → left for review (R1) |
| Marketplace publish | ALLOWED kinds only (methodology, jurisdiction rule set, pipeline, tool); blueprints/containers/results refused; McpEntity ingest nodes never announced; peer-imported items not echoed; manifests only (name, hash, keywords) | `network/mod.rs:117,1043–1060`, `amt_loop.rs:1917–1965` | — |
| Marketplace accept | peer allow-list ("we don't accept anything not allowed"), local have/conflict/new classification, advisory review (never acts), operator `download` decision, hash RE-VERIFY, JSON validation; jurisdiction files STAGED only; tools/pipelines approval-only | `peer_import.rs:1–16,81,301–330,380`, `offer_review.rs:118,194` | imported methodology bodies get no placeholder/empty-rule check before create (R2) |
| AMT expansion | rides the walk's gates; new items grafted as child tasks (`wo_*`), completed final | `amt_loop.rs`, `task/mod.rs:2279` | — |
| Rerun/graft | completed = FINAL; rerun = new task instance (`rerun_of`), graft rejected on completed | `task/mod.rs:2254–2330` | — |
| Keyword validation | strength-N yes/no, unanimity to drop, evidence-gated, settlement kill switch | `keyword_loops.rs:82`, `keyword_program.rs` | — |

No guardrail was ADDED in code this pass: each gap above needs either a model judgment (a new call site, which the zero-shot guide registers) or a refusal rule the operator has not ruled on; both are recorded below rather than invented.

## 3. Staged review flow of a blueprint once created

| # | Stage | What it does | Where |
|---|---|---|---|
| 1 | Draft | model drafts steps; non-provider steps coerced to pipeline 9 (provider steps keep provider); `missing_capabilities` kept | `stages.rs:1300,1832–1850` |
| 2 | Rule gate | ONE call per round: applicability + step→rule fidelity (strength rounds; yes/no; a failed call drops nothing); all judgments recorded as samples | `judge_rule_applicability` `stages.rs:824`, `rule_gate_rounds` `:923`; called at `:1403` (reused blueprint) and `:2120` (new) |
| 3 | Citation audit | `audit_rule_refs` — step rule refs checked against the real gathered rules | `stages.rs:1016`; called `:1404`, `:2121` |
| 4 | Coverage | uncovered rules → `rule_uncovered` event + `methodology_refinement_candidates.jsonl` | `stages.rs:1056` → `meta_loop.rs:540` |
| 5 | Tool binding / gaps | `reconcile_tool_bindings` (steps addressed by position); gaps recorded | `stages.rs:20,2122` |
| 6 | Persist | blueprint container written | `persist_blueprint_container` `stages.rs:590,2133` |
| 7 | Capability closure | each gap JUDGED (strength-5); real need → methodology on the spot, then tool (E30), re-plan while progress is made | `close_capability_gaps` `stages.rs:1074`, loop `:303–318` |
| 8 | Simulation | zero-shot simulation, feasibility, may add gaps | `stage_4_zero_shot_simulation` `:2161,2361` |
| 9 | Consciousness gate (if on) | decision | `:371` |
| 10 | Jurisdiction full gate | judges the resolved plan | `:381` → `jurisdiction.rs:503` |
| 11 | Execution | step gate + call gate per step | `:3384`, `mcp.rs:162` |
| 12 | Refinement (meta loop, async) | aggregate candidates → unanimous split → focused methodology → retire moved rule | `meta_loop.rs:87–90,177,227,406` |

## 4. Stale / dead / questionable code NOT superseded — for review (nothing deleted)

Carried from the build-readiness pass (plan §10): `load_methodology_rules_text`, `on_item_announced`, `publish_to_marketplace`, `process_batch_sync`, `on_methodology_created`, `on_blueprint_created`, `on_container_created` (docs-registered or `pub`; no superseder identified).

New in this audit:
* **F1** `orchestrator/work_order.rs` / `task/mod.rs` `node(..., depth > 64)`: a fixed limit in the tree read (cycle guard). Doctrine says no fixed caps; replace with a visited-set cycle check — needs the operator's nod (behavioural).
* **F2** `keyword_settle.rs:96,42` (`graph_terms_max` 256, `rollup_terms_max` 512) and `span_out.rs:99` (`DEFAULT_FETCH_CAP` 400): documented host-memory guards with env overrides — kept; listed because they are fixed numbers.
* **F3** `#[allow(dead_code)]` in `consciousness/assistant.rs:49,56`, `orchestrator/i_loop.rs:65`, `orchestrator/mod.rs:1162,1186,1196,1405,1417`, `types/modality.rs:10`: fields captured "for consumers not yet wired"; no superseder known — review whether each consumer exists.
* **F4** `zsei/search.rs` `scan-legacy` / `exact` strategies and `k_registry` `keywords-only` relevance: kept by policy ("algorithms are never removed, only superseded") — not stale.
* **F5** `orchestrator/amt.rs` legacy per-chunk zero-shot loop (`:152,666`) and `adapters.rs:216` flat-container normalizer: live fallbacks, not dead.
* **F6** pipeline 36 `TaskViewer` DEPRECATED in `bootstrap.rs:826` (filtered by `!p.deprecated`): superseded by the merged viewer; entry kept for id stability.
* **F7** `NetworkConfig::auto_broadcast_sync` alias is now redundant with both explicit switches defaulting on; keep for old configs.
* **R1** created-tool publish has no leak check beyond the prompt (see §2). **R2** imported methodology has no placeholder/empty-rule check (see §2).
* **R3** `OZONE_MIN_AVAIL_MB` guards the queued-task executor only (see §2).

## 5. Third-party interaction note

Everything here was read from code; no EU or local host was touched, nothing compiled. The verification of each row above is a first-compile / first-walk job in the plan (§7).
