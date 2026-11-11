# Tests, dry-runs and script-like code inside the production tree — audit (2026-11-11)

Operator rulings applied: tests/dry-runs must not overreach or live inside production code; scripts belong outside the code; code is production-deployable; verification is done as a THIRD PARTY through Ozone-Studio's public surface, not through code-edit shortcuts. **Nothing was built, compiled or run** to produce this (counts are `grep` of git-tracked files; the one edit is parse-level only). No EU change.

Scope read: `src/`, `assets/pipelines/**`, `shared/`, `tests/`, `ui/src`.

## 1. Totals (git-tracked, `#[test]` + `#[tokio::test]`)

| area | files with tests | tests |
|---|---|---|
| `src/` (host) | 52 | 270 |
| `assets/pipelines/modalities` | 31 | 264 |
| `assets/pipelines/general`+`shared` | 6 | 28 |
| `shared/contracts` | 4 | 24 |
| `tests/` (integration) | 2 | 5 |
| `ui/src` | 0 (no test files, no test script in `ui/package.json`) | 0 |

(The BUILD_AND_TEST plan's "about 291 lib tests" counts differently; use the real `cargo test` total after the build.)

## 2. Classification

### A. Legitimate unit/integration tests — KEEP (Rust convention: `#[cfg(test)] mod tests` beside the code, compiled out of release)
Per-file list (host `src/`; line = first `#[cfg(test)]`, then test count, then mock/fake/stub mentions):

| file:line | tests | mock-ish mentions |
|---|---|---|
| `src/config/mod.rs:1079` | 2 |  |
| `src/consciousness/assistant.rs:914` | 8 |  |
| `src/consciousness/proposals.rs:671` | 19 |  |
| `src/consciousness/response_record.rs:86` | 1 |  |
| `src/context_budget.rs:300` | 5 |  |
| `src/context_mirror.rs:333` | 6 |  |
| `src/context_needs.rs:240` | 3 | 3 |
| `src/cross_modal_index.rs:111` | 2 |  |
| `src/graph_events.rs:148` | 7 |  |
| `src/grpc/mod.rs:6864` | 9 |  |
| `src/integrity/mod.rs:395` | 2 |  |
| `src/keyword_program.rs:1138` | 14 |  |
| `src/k_registry.rs:235` | 2 |  |
| `src/mcp_graph.rs:923` | 10 |  |
| `src/mcp.rs:699` | 8 |  |
| `src/model_ledger.rs:952` | 5 |  |
| `src/monitor/mod.rs:135` | 2 |  |
| `src/network/mod.rs:1806` | 3 |  |
| `src/orchestrator/actors.rs:834` | 12 | 7 |
| `src/orchestrator/adapters.rs:527` | 3 |  |
| `src/orchestrator/amt_candidates.rs:215` | 6 |  |
| `src/orchestrator/amt_loop.rs:1122` | 13 | 30 |
| `src/orchestrator/amt.rs:4049` | 5 | 1 |
| `src/orchestrator/awaiting.rs:229` | 2 |  |
| `src/orchestrator/chat_store.rs:486` | 2 |  |
| `src/orchestrator/decision_review.rs:792` | 10 | 3 |
| `src/orchestrator/dispatch.rs:120` | 4 |  |
| `src/orchestrator/entry_gate.rs:184` | 4 |  |
| `src/orchestrator/graph_fanout.rs:371` | 5 |  |
| `src/orchestrator/instrumented.rs:120` | 2 |  |
| `src/orchestrator/jurisdiction.rs:935` | 17 |  |
| `src/orchestrator/keyword_settle.rs:779` | 8 |  |
| `src/orchestrator/meta_loop.rs:1466` | 2 |  |
| `src/orchestrator/mod.rs:5694` | 4 | 7 |
| `src/orchestrator/offer_review.rs:202` | 3 |  |
| `src/orchestrator/peer_import.rs:505` | 8 |  |
| `src/orchestrator/request_terms.rs:117` | 5 |  |
| `src/orchestrator/ripple_class.rs:337` | 5 |  |
| `src/orchestrator/run_spans.rs:388` | 4 |  |
| `src/orchestrator/span_out.rs:587` | 3 |  |
| `src/orchestrator/stages.rs:4420` | 4 |  |
| `src/orchestrator/tool_binding.rs:551` | 2 |  |
| `src/orchestrator/tool_creation.rs:234` | 4 |  |
| `src/orchestrator/tool_routing.rs:431` | 3 |  |
| `src/pipeline/gate.rs:249` | 4 |  |
| `src/task/mod.rs:2454` | 5 |  |
| `src/task/store.rs:88` | 2 |  |
| `src/trace.rs:150` | 3 |  |
| `src/types/container.rs:1726` | 2 |  |
| `src/types/methodology.rs:91` | 1 |  |
| `src/zsei/hooks.rs:989` | 2 |  |
| `src/zsei/storage.rs:956` | 3 |  |

Pipeline/shared/integration tests: 31 modality files (largest: `uidocument/src/lib.rs` 20, `audio/main.rs` 17, `image/main.rs` 15, `3D/readers.rs` 15, `uidocument/src/android.rs` 15, `geospatial/geometry.rs` 14, `control/main.rs` 14, `video/main.rs` 13, `shared/contracts/keyword_model.rs` 14), `assets/pipelines/general/{methodology_create,package_link,voice}`, `assets/pipelines/shared/semantic_relations.rs` 11, contract tests `assets/pipelines/{general/context_viewer,general/task_manager,modalities/text}/tests/contract.rs`, root `tests/executor_l2.rs` (2) and `tests/remote_dispatch.rs` (3). Re-run with: `git ls-files '*.rs' | xargs grep -cE '#\[(tokio::)?test\]'`.

### B. In-test scaffolding that conflicts with the "no mocks" doctrine — KEEP AS UNIT SCAFFOLDING, NEVER AS VERIFICATION EVIDENCE
`MockExecutor`/`MockStore`/`MockZSEI`/`FakeStore`/`MockInner`/`StubExecutor` appear only inside `#[cfg(test)]` (after the move in §4): `src/orchestrator/amt_loop.rs` (MockStore, MockExecutor, ~30 mentions), `actors.rs` (MockStore), `mod.rs:5699,5771` (MockExecutor, MockZSEI), `decision_review.rs:797` (MockInner), `context_needs.rs:244` (FakeStore), `amt.rs:4085` (RecordingStore with `project_fixture`), `assets/pipelines/modalities/text/main.rs` (MockExecutor, StubExecutor). They isolate a function from a model/network; they are not proof that the system works. **Rule for reports:** a green `cargo test` is a unit-level signal only; "it works" is evidenced by §5 (third-party run against the real host). Replacing them with real-host checks is not recommended in-tree (it would pull network/model calls into `cargo test`); the real-thing coverage is `tools/e2e/*` + `tools/third-party-verify/*`.

### C. Script-like / dry-run / harness code that should NOT be in production code
Searched: `dry_run`, `self_test`, `demo`, `fixture`, `harness`, `smoke`, `--selftest`, `OZONE_TEST*`/`OZONE_DRY*`/`OZONE_MOCK*`, `[[example]]`/`[[test]]`/`benches`, debug/test HTTP routes (full route table in `src/grpc/mod.rs:4925-5100` read).
* **Found and fixed (trivially safe):** `assets/pipelines/modalities/text/main.rs:8103` — `struct StubExecutor` + `impl PipelineExecutor` were test-only (single use at `test_reconstruct_from_chunks`) but sat OUTSIDE `#[cfg(test)]`, i.e. compiled into the production `text` binary. Now `#[cfg(test)]`. (Uncompiled edit; `text` must be rebuilt per the plan anyway; if the first `cargo check` complains, revert these two attributes.)
* **No other script-like code found in production paths.** The CLI dispatch in `src/main.rs`/`src/cli.rs` is the shipped operator CLI (a feature, not a harness). Every pipeline `main()` reading `--input`/stdin is the host↔pipeline protocol, not a test harness. There are no debug/test/probe HTTP routes. E2E suites, demos and fixtures already live outside the code: `tools/e2e/*` (18 suites + `fixtures.mjs`, `rerun-all.sh`), `tools/browser-mcp/demo_ozone_ui.mjs`, `tools/walk-scorecard`, `tools/keyword-measure/*`.

### D. Risky production-embedded paths — LISTED FOR REVIEW (not changed)
| item | where | why it is listed | suggested handling |
|---|---|---|---|
| Forged "demo" auth key + signature | `ui/src/services/store.ts:289-306` | The UI store derives a "public key" from the device id and a "signature" by XOR in the shipped client ("simplified for demo… in production use HKDF"). Production code that fabricates credentials is the classic demo-in-production hazard; if the host verifies Ed25519 signatures it cannot succeed, so either the path is dead or the host accepts it. | Operator decision: replace with the real device-key flow used by `tools/ozone-cli`/`tools/e2e/lib.mjs` (Ed25519 over the challenge) or remove if Electron main handles auth. Needs a read of `window.ozone.auth.authenticate` in `ui/electron` first. |
| `dry_run_first` | `src/types/integrity.rs:131` | A rollback-request flag with no reader anywhere in `src/` (only the definition). Superseder not identified. | Left for review per the dead-code rule (do not delete without identifying a superseder). |
| Debug-build fallbacks in the executor | `src/pipeline/executor.rs:675-715` | Production host will run a `target/debug` or crate-local binary if the shared release one is absent. The code comment says this is a deliberate dev fallback after the shared-release candidate (documented, 2026-10 stale-binary incident). | Keep; consider a `OZONE_ALLOW_DEBUG_PIPELINES` guard only if an operator wants prod to refuse debug binaries. |
| `ui/src/graphEventClient.ts:515` | `console.debug` of every graph event | Dev logging in the shipped UI (also `DiscoveryEffects.tsx:199,207`). | Low risk; gate behind a dev flag if noise matters. |
| Stale agent worktrees | `.claude/worktrees/agent-*` (9, gitignored) | Duplicate full source copies inflate every repo-wide grep (they hide as "dry_run" hits). Not in git. | Operator: `git worktree list` then remove those no longer needed. |
| No UI test runner | `ui/package.json` | `build` is `vite build` (no `tsc`); the new components have never been type-checked. | Add `"typecheck": "tsc --noEmit"` as a script (a build-tooling change; see plan §3) — not done here. |

## 3. What belongs where (relocation plan)
1. In-code: only `#[cfg(test)]` unit tests (A) with their scaffolding (B). Nothing else.
2. `tools/e2e/*` — the existing real-host suites (some write inside the repo and may spend model calls). Keep.
3. `tools/third-party-verify/*` (NEW) — read-mostly outside-client verification, defined in §5.
4. Operator one-offs (backfills, sync, measurements) stay in `tools/keyword-measure`, `tools/sync`.
Nothing needs to be moved out of `src/` today (§C: only the StubExecutor attribute).

## 4. Edits made
* `assets/pipelines/modalities/text/main.rs`: `#[cfg(test)]` on `StubExecutor` and its impl (3 lines + comment). No other change.

## 5. Third-party verification suite (created, NOT RUN)
`tools/third-party-verify/` (README there). Outside-client rule: public HTTP surface only, device-key session (same as `tools/ozone-cli`), no code or data-dir reads, no mocks, every row shows the value actually read, absent = SKIP with reason. Evidence → `docs/evidence/third-party-verify/<date>-<suite>.md`.

| script | reads | mutates |
|---|---|---|
| `01-surface.mjs` | /health, session auth, reachability of the read routes | no |
| `02-work-order.mjs` | /task/list, /order/work/:id (tree shape, no offer items) | no |
| `03-gaps-and-refinement.mjs` | /capability/gaps, /methodology/refinement (no confidence fields, honest counts) | no |
| `04-call-graphs.mjs` | workspaces→projects→ModalityGraph containers + code-graph Calls edges via /zsei/query (answers: are graph containers present without a prompt?) | no |
| `05-marketplace-views.mjs` | /marketplace/overview (policy, offers classification, no blueprint body) | no (never calls offers/decide) |
| `06-walk.mjs` | ONE real /orchestrate walk, then the work-order tree and gap delta | yes — opt-in `OZONE_TPV_WALK=1` (spends model calls) |
| `07-tools-and-coordination.mjs` | /mcp/tools, /mcp/usage, /coordination/* (created tools not running, auto_start=false) | no |
| `run-all.sh` | runs 01–05 + 07 | no |

Ordering vs `docs/BUILD_AND_TEST_PLAN_2026-11-10.md`: after §5 (install+start, `active_tasks==0`, deploy-OFF drop-in) and BEFORE §7 (first real walk) run `01`,`03`,`05`,`07` (baseline on the deployed build, expect empty gaps/refinement candidates). Run `06-walk` as the §7 walk; then `02`,`03`,`04` to read the effects back. The existing §8 suites (`tools/e2e`) follow. Only after the §9-style enablement steps (one switch at a time) re-run `03`/`07`.

Known limits: the `02/03/05/07` field names were taken from the handlers in `src/grpc/mod.rs` (`get_work_order`, `get_capability_gaps`, `get_methodology_refinement`, `marketplace_overview`) and the UI types, not from a live response — the first run may FAIL a shape check because a name differs; treat that as a finding about either the script or the endpoint, and fix the script from the real payload (never loosen a check to pass). There is no dedicated call-graph HTTP endpoint: `04` reads graph containers via `/zsei/query` like the UI does.
