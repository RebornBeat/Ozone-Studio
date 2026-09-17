> The test contract for the living graph: every graph in Ozone-Studio gets
> repeatable, honest tests — captured facts only (real registrations, real
> edges, real persisted bytes), never fabricated assertions.
>
> Legend: [x] exists · [ ] to build · (live) verified live this session
> Owner tags route work through the host task system (42-57 arc pattern).

Last updated: 2026-09-15 — initial plan (coverage survey: zsei core 2 tests,
jurisdiction 0, context_mirror 0, mcp 0, math 0, code 1, text 5, orchestrator
8, task 3).

---

## 1. Jurisdiction graph (root 7 — 41 scopes, 40 edges live)

- [x] **T-J1 Idempotent self-heal** — boot twice on unchanged files → identical container set (scope-keyword dedupe holds), registration count 0 on re-run. Codified by extracting the pure decision logic (`compute_new_jurisdiction_registrations`, src/lib.rs) out of the boot-time block; test in `src/orchestrator/jurisdiction.rs::graph_tests`.
- [x] **T-J2 Edge traversal** (CC's pending live verification, codified) — a national ruleset's traversal reaches its EU baseline via exactly one real `Relation` edge; assert edge count + discovered_via provenance. Built a REAL on-disk `ContainerStorage`+`TraversalEngine` fixture (not a mock) exercising the actual traversal engine, including a distractor container of a different type to prove the type filter, not just BFS reachability.
- [x] **T-J3 Region detection** — 2-of-3 signal agreement (timezone/locale/IP fixtures); explicit config value always wins; disagreement → no auto-fill. Split across two files: `HardwareRegionSignals::agreed_region` fixture tests in `jurisdiction.rs::graph_tests` (pure logic, no refactor needed), config-wins-over-detection test in `src/config/mod.rs::graph_tests` (private method, same-module test required).
- [x] **T-J4 Content honesty** — every registered jurisdiction JSON: non-empty rules, real citations, `action: Log` throughout for auto-populated (`national/`) content, disclaimer present. `global.json` is the one explicit, human-reviewed exception (see below) — exempted from the Log-only check by filename, not by any "hand-curated" heuristic. Real filesystem walk over `assets/jurisdiction/`, not a fixture (41 files).
- [x] **T-J5 Edge idempotency** — the 40-edge wiring re-run does not duplicate edges (matches CC's `changed` flag pattern). Codified by extracting `compute_jurisdiction_edges_to_add` (src/lib.rs) — same function proves both the edge shape (T-J2's provenance half) and idempotency (calling twice with the first call's output merged in returns empty the second time).

**T-J4's real finding went through two full rounds with the user before landing — worth recording precisely, not just the end state.** Round 1: the test's first draft asserted `action: Log` universally and correctly failed against `global.json`'s real Warn/RequireConfirmation/Block rules (discrimination, child exploitation/abuse, self-harm — hand-curated from UDHR/CRC articles, predating the Log-only convention). The fork exempted global.json rather than touch safety content unilaterally — reasonable, but surfaced to the user rather than left as a unilateral call. The user's first answer: downgrade all 7 to Log for store-wide consistency — implemented. Round 2, on restart: the user reconsidered, correctly pointing out this could regress real working functionality. Investigating found the real technical picture neither side had checked yet: **`Block` was the only action with actual differentiated runtime behavior** (a real early-return stopping the request, `jurisdiction.rs`) — Warn and RequireConfirmation did nothing different from Log at all. So the Round-1 downgrade was a real, silent regression on Block specifically, not a cosmetic relabel. Resolved: **restored global.json's original 7 actions, then built genuinely new, real enforcement for Warn (a surfaced warning) and RequireConfirmation (a real decision_gate-pipeline review — the same mechanism the Consciousness Gate already uses)** — see the new section below. `OrchestrationResponse.jurisdiction_gate` was also added — the whole `JurisdictionGateResult` had been write-only on internal state and never reached the API caller, even for an actual `blocked` request, until now. New methodology 30 (Enforcement Actions Need Real Behavior, Not Just Labels) captures the core lesson. Also applied the same absolute-path defensive fix ZCode found in `amt_loop.rs` to `load_jurisdiction_rules`'s own `object_store_path` join in `src/orchestrator/jurisdiction.rs` — a **third** occurrence of the same bug class turned up separately in `amt.rs`'s `load_methodology_rules_text` while building T-62's test (see task 62 below).

### Real enforcement now exists for Warn and RequireConfirmation (new this pass, not in the original plan)
- `categorize_jurisdiction_matches` (pure, extracted for testability): routes each matched rule by action — Block sets `blocked`, Warn pushes a real surfaced warning string, RequireConfirmation is returned separately for review, Log is a no-op. 2 tests cover full routing plus the zero-match case.
- `resolve_confirmation_reviews` (async, also extracted so it's testable without mocking the whole rule-loading/store chain): for each RequireConfirmation match, calls the real decision_gate pipeline (#39) — the exact mechanism `stage_5_consciousness_gate` already uses — with a jurisdiction-specific framing (matched condition + legal source + truncated prompt). A genuine `Decline` sets `blocked`; `Proceed` doesn't; a failed review call is recorded honestly (`ReviewFailed`) and fails open rather than fabricating either an approval or a block. 3 tests: Decline blocks, Proceed doesn't, a failed call fails open and is visible.
- All 5 new tests real, `cargo test --lib`: 59/59, real exit 0. `cargo build --release`: real exit 0.

## 2. AMT graphs (per-task abstraction trees)

- [x] **T-A1 Branch coverage** — one step per AMT branch, multi-intent never collapses (built this session; codify as regression test).
- [x] **T-A2 Parenting** — AMT container parents to the task's project (the hardcoded-parent_id-0 bug class stays dead). 2 tests in `orchestrator::amt::tests` (real project_id → real parent, no project_id → 0 fallback), asserted against a recording mock store, not just "the call succeeded."
- [x] **T-A3 Re-expansion loop** — unverified branch → real trigger → capped retries → fallback model escalation, and specifically that escalation CYCLES through every fallback candidate rather than clamping to the last one (the exact bug class fixed earlier this session). `orchestrator::amt_loop::tests::reexpansion_retry_cycles_through_fallback_candidates_not_clamps` calls `try_reexpand_one` directly across 4 successive attempts with 2 fallback candidates and asserts the model sequence is `[none, a, b, a]` — proving the wrap-around, not just "it retried."
- [x] **T-A4 Persistence** — amt_summary survives restart on the task record. `task::tests::amt_summary_survives_restart` constructs a real `TaskManager`, sets amt_summary, then constructs a SECOND `TaskManager` against the same storage_path (a genuine restart simulation — `TaskManager::new` always calls `load_from_disk_sync`) and asserts the summary round-trips through real disk persistence.
- [x] **T-A5 Cross-references** — AMTRelation expands a step's context with related-branch content. The reconciliation logic (`collect_relationship_targets`/`find_amt_node_by_id`/`related_chunk_indices`) was previously only reachable inside `stage_3_blueprint_assignment`'s ~400-line body — extracted to module-level free functions in `stages.rs` (same logic, no behavior change) so 3 real tests in `orchestrator::stages::amt_relation_tests` could exercise it directly: a branch with a real relationship pulls in the target branch's chunks, a branch with none pulls in nothing, and a relationship on a deeper descendant (the multi-intent tree shape) is still found.
- [x] **T-62 Re-expansion prompt is genuinely guided** (task 62, not in the original block — a real gap found by direct inspection: the re-expansion call built its prompt from only root/target content strings, never reading the target node's real `methodology_ids`/`relationships`, even though both are populated and the mechanism to use them — `load_methodology_rules_text` — already works correctly elsewhere in `stages.rs`). Fixed: `try_reexpand_one` now looks up the target node itself (not just its content), injects real methodology decision-rules text for every `methodology_id` it carries, and surfaces related-branch content via its `relationships`. `orchestrator::amt_loop::tests::reexpansion_prompt_includes_methodology_guidance_and_related_branches` captures the real prompt sent to the mock executor and asserts both a real injected decision-rule string and a real related-branch content string actually appear in it — not just that the call succeeded. Caught 2 real bugs while writing this test: an `AMTRelationType` enum-variant typo in the test fixture (`RelatedTo` vs the real `RelatesTo`), and a genuine **third occurrence** of tonight's absolute-path garbage-join bug class, this time in `load_methodology_rules_text` (`amt.rs`) — every prior real call site happened to pass a relative path so it was latent; fixed the same way as the other two.

## 3. Text modality graphs (root modality tree)

- [x] **T-T1 Chunking** — no-overlap boundaries, char-boundary floors/ceils (existing 5 tests; keep green).
- [ ] **T-T2 Container parenting** — chunk graphs parent to project when project_id ≠ 0, root fallback otherwise (same rule as code).
- [ ] **T-T3 Cross-process retrieval** — CLI invocation sees previously-stored graphs. **KNOWN BROKEN** — fix (math's reference impl) then this test proves it; currently asserts the empty-cache reality and is expected-fail-documented.
- [ ] **T-T4 Hierarchy** — sentence → paragraph → section containers with correct parent chains.
- [ ] **T-T5 Overlap resolution** — punctuation-scanning fallback scenarios (ending-mid-sentence, no-terminator chunks).

## 4. Code modality graphs

- [x] **T-C1 AST parse** — functions/classes/imports become real containers (live-verified; codify).
- [ ] **T-C2 Parenting** — project_id parenting, root fallback.
- [ ] **T-C3 Cross-process retrieval** — same gap as T-T3.

## 5. Math modality graphs (the reference implementation)

- [ ] **T-M1 Container + relationship** — expression analysis → container with methodology relationship (the live-verified path, as a test).
- [ ] **T-M2 Cross-process retrieval** — the reference all other modalities copy.

## 6. Coordination graph (/SharedContext — src/context_mirror.rs)

> **CODIFIED 2026-09-16** — T-CO1..CO5 live as `context_mirror::tests`
> (5 passing), T-G1..G3 as `graph_events::tests` (6 passing, incl. CC's),
> T-U1..U4 as `mcp::tests` + `task::tests` (11 passing). Suite: 63/63.
> Remaining plan-only: T-G4 (WS frames integration), T-I1..I4
> (cross-graph, blocked on 57/56), T-T3/C3/M2 (cross-process batch).

- [x] **T-CO1 Scoping** — workspace note gets `ws:<id>`, global gets `scope:global`, project requires workspace. `mirrors_carry_scope_keywords` — real temp ZSEI store, asserts the actual keyword strings on the stored container.
- [x] **T-CO2 Claim dedupe** — same file claimed N times → exactly one container. `claim_dedupe_same_file_one_container`.
- [x] **T-CO3 Coercions** — global claim → workspace; project-without-workspace → workspace; unknown scope → workspace (never silent global). `scope_coercions`.
- [x] **T-CO4 Body persistence** — object_store_path file exists and contains the full event JSON. `body_persisted_via_object_store_path`.
- [x] **T-CO5 Note → graph auto-mirror** — note_add through the MCP produces a CoordinationEvent container. `note_add_produces_a_real_coordination_event_container`.

**Real bug found while writing this block, not fixed here (storage-core, flagged for a later pass)**: the test fixture found `mmap_enabled: false` silently no-ops `ContainerStorage::store_global`'s byte writes — a real data-loss bug for any caller running with mmap disabled. Worked around in the tests by using `mmap_enabled: true` (the supported path); the underlying bug is real and unfixed.

## 7. Graph ripple (src/graph_events.rs)

- [x] **T-G1 Every write emits** — create/update/delete/link each produce one event at the query choke point; reads emit none. `real_create_container_emits_a_correctly_provenanced_event` plus `publish_subscribe_roundtrip`.
- [x] **T-G2 Provenance capture** — type + scope keywords captured pre-write (CreateContainer carries the container's own keywords). Covered by the same `real_create_container_emits_a_correctly_provenanced_event` test.
- [x] **T-G3 visible_to rules** — global subscribers see all; ws/proj subscribers see globals + own scope; empty-scope events only reach global. `global_subscriber_sees_everything`, `own_scope_events_match`, `global_event_reaches_all_scopes`, `empty_hub_subscribe_does_not_panic`.
- [ ] **T-G4 WS forward** — connected socket receives `graph_event` frames (integration). Not covered — genuinely an integration test (needs a live WebSocket connection), correctly out of scope for this unit-test pass.

## 8. Context objects (per-step context provenance)

- [x] **T-X1 Stage 7 persistence** (built; codify) — every step_contexts entry lands on the task record.
- [ ] **T-X2 Find-or-create** — update_step_context creates missing steps, overwrites stale context.
- [ ] **T-X3 API exposure** — GET /task/get returns context_assembled + context_sources per step.

## 9. MCP usage + coordination lifecycle (src/mcp.rs, /task/*)

- [x] **T-U1 Ledger** — per agent/day/tool counting; day rollover resets. `ledger_counts_per_tool`, `ledger_isolates_agents`, `day_rollover_boundary_is_correct`.
- [x] **T-U2 Gate** — over-limit refusal counts the call and returns allowed:false. `ledger_gate_refuses_over_limit`.
- [x] **T-U3 Lifecycle guard** — /task/update touches only source-tagged tasks; host-executed tasks refused. `invoke_stdio_tool_delegates_and_meters`, `invoke_unknown_tool_fails_with_usage_recorded`.
- [x] **T-U4 Reconciliation** — source-tagged + assignee tasks survive restarts queued; others honestly interrupted (live-verified; codify).

## 10. Cross-graph integration

- [ ] **T-I1 Attachment linking** (blocked on task 57) — attachment graph links to relevant containers; role labels on edges.
- [ ] **T-I2 Jurisdiction traversal integration** — orchestration prompt mentioning a regulated topic pulls the right jurisdiction containers via traversal (blocked on T-J2 + task 56 wiring).
- [ ] **T-I3 File links** — file_link → File container → project relationship via ZSEIQuery::LinkFile (currently flat).
- [ ] **T-I4 Integrity** — blake3 tampering detection on a graph container (integrity monitor has real detection; create_snapshot needs a call site first).

---

## Execution order

1. **zcode**: T-CO1..CO5, T-G1..G3, T-U1..U4 (pure unit territory — my modules, no restart needed for most)
2. **claude-code**: T-J1..J5, T-A2..A5 (its jurisdiction/AMT domain; several ride task 56's live verification)
3. **Both, after task 57 + traversal wiring**: T-T3/C3/M2 cross-process batch, T-I1/I2 integration batch

Routed through host tasks; check the queue before starting any block.
