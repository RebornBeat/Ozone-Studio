# Call graphs in the UI, as graph containers (2026-11-11) — EDITED ONLY, nothing built or tested

Operator ask: see the call graphs **without running a prompt**; on a prompt, the dynamic UI per modality (e.g. code) shows the work **alongside** the call graphs; evidence them as graph containers; and: are those containers created on workspace load, or should they be?

## 1. Answer: were they created on workspace load?

There were two different things called "call graph". Neither existed as a container made at load.

| What | State before this change | Created when |
|---|---|---|
| **The system's own call graphs** (walk stages → model-call sites; loops; call routes + guardrails) | Only prose: `ZERO_SHOT_CALL_REGISTRY.md`, `CALL_ROUTE_REGISTRY.md`. No machine-readable form, no container, no endpoint, no UI. | **Never.** |
| **A project's code call graph** (`Calls` edges from the code modality) | `ModalityGraph` containers under the project, rendered by `ui/src/views/engines/CodeCallGraph.tsx` (same-file resolution only, banner says so). | **Only when a `/orchestrate` walk attaches the file** (`orchestrator/graphs.rs`, stage 2 graph creation). NOT on workspace load, NOT on file link, NOT on a file edit. `CHECKLIST.md` L2714 already recorded the missing edit → re-analysis trigger. |

Should they be created automatically? **Yes for the system graphs** — they are system facts, identical for every workspace, cost no model call, and must be visible before any prompt. Done below (created/refreshed at host boot). **For project code graphs: yes in principle, but not done here** — it means running the code pipeline on workspace load / on a file-beacon change (a real actor, a real cost, and the "drive it through /orchestrate" doctrine applies). Recommended design is in §5; it needs an operator decision.

## 2. Design (what was edited)

**One source of truth, generated, never hand-copied.** `tools/call-graph/extract_system_call_graph.mjs` reads `src/orchestrator/**` (+ `consciousness/assistant.rs`) and the two registries and writes `assets/call_graphs/system_call_graph.json` (deterministic, no timestamps; `--check` exits 1 on drift; `--stdout` prints). I ran it once to produce the asset (read-only over the repo, no build). Current extraction:

| graph | nodes | edges | content |
|---|---|---|---|
| `walk` | 51 | 50 | 14 stages in `execute_stages` order (`order` edges are not drawn as edges: the `order` attribute carries it) → the functions that make a model call (`reaches`, with the shortest `via` path) → 15 labelled call sites + 5 strength-N vote sites; call sites no stage reaches by name are KEPT, marked `attached:false` (2 today) |
| `loops` | 15 | 10 | each loop file → its 10 labelled `capture_loop_model_call` sites |
| `routes` | 50 | 36 | 37 routes (R1–R37) and 13 guardrail layers (G1–G13); `guarded_by` edges from the `G<n>` names in each registry row |

Honest limits (also in the asset's `notes` and on screen): static and name-based; a function name defined more than once (26 today) takes no part in reachability and is never guessed; a name-based edge must be `self.x(` / `Self::x(` / bare `x(` (never `value.x(`); fn bodies are brace-matched. `in_registry:false` flags a label missing from `ZERO_SHOT_CALL_REGISTRY.md` — **2 today: `file_role_classification`, `methodology_compliance_check`** (registered under other wording; an operator/registry fix, not changed here).

**Containers (created at workspace load = host boot).** `src/orchestrator/call_graph.rs::ensure_system_call_graphs`, spawned once from `src/lib.rs` next to the other boot actors (best-effort, never blocks boot): one `Derived` container per graph ("Call graph: …", `/CallGraphs/<key>`, parent 0, keywords `call graph`, `<key> call graph`, `orchestration`), content body at `<data_dir>/call_graphs/<key>.json` via `object_store_path` (readable with `GetContainerContent` like every other body). **Idempotent:** ids + blake3 body hashes live in `<data_dir>/call_graphs/containers.json`; container still exists + body unchanged → left alone; body changed → rewritten in place (same container); container deleted → recreated. Writes go through the normal create door, so the keyword write gate and creation capture (R27/R28) apply. Asset missing → logs and reports `available:false`, invents nothing.

**Endpoints (read-only, `src/grpc/mod.rs`).**
- `GET /callgraph/system` — the graphs from the asset (works before the containers exist) + each graph's `container_id` (`null` until boot created it).
- `GET /callgraph/live?task_id=N | trace_id=T | latest=1` — the executed calls (`capture/call_spans.jsonl`) and stage events (`walk_metrics.jsonl`) of ONE trace, streamed by exact trace id (no row cap), plus `by_call_site` {runs, failed, last_latency_ms} and `unlabelled_calls`. `task_id` resolves the trace from the task's `inputs.trace_id` (already written by `task/mod.rs`); a task with no trace answers `trace_id:null` — never another walk's trace. `latest=1` = the last trace in `walk_metrics.jsonl`.
- Registered as **R38** in `docs/CALL_ROUTE_REGISTRY.md` (a route with no row is a bypass).

**UI.** `ui/src/components/CallGraphView.tsx` (+ `ozoneClient.ts` types/fetchers, `App.css` `.ocg-*`): tabs per graph; walk = stage → fn (with `via`) → call sites; loops; routes with guardrail chips. Static always; live overlay polled every 3 s when a walk is chosen: call sites that ran show `ran ×N` / `failed ×M` / last latency, stages with any ran site are marked, the walk's stage events show as chips. Absence is not a verdict (a site that did not run shows nothing). Mounted in three places:
1. **Engines → "Call graphs" tab** (no prompt needed; follows the latest walk). Inserted second, so `ENGINES[0]` (default / fallback) is unchanged.
2. **Engines → Code tab**, a collapsible "Orchestration call graph (live overlay)" under the project's `CodeCallGraph` — the walk working alongside the code view.
3. **Task detail**, under the work-order tree, overlaying THAT task's walk.

No confidence numbers, scores or percentages anywhere; edges present/absent, calls ran/did not.

## 3. Files

New: `tools/call-graph/extract_system_call_graph.mjs`, `assets/call_graphs/system_call_graph.json`, `src/orchestrator/call_graph.rs` (4 unit tests, not run), `ui/src/components/CallGraphView.tsx`.
Edited: `src/orchestrator/mod.rs` (module decl), `src/lib.rs` (boot spawn), `src/grpc/mod.rs` (2 handlers + 2 routes + query struct), `ui/src/ozoneClient.ts`, `ui/src/panels/enginesRegistry.tsx`, `ui/src/components/TaskDetailPanel.tsx`, `ui/src/App.css`, `docs/CALL_ROUTE_REGISTRY.md` (R38).

## 4. Verification status (honest)

- `rustfmt --edition 2021 --emit stdout` parses `call_graph.rs`, `grpc/mod.rs`, `lib.rs`, `orchestrator/mod.rs`: OK. `ts.transpileModule` parses the 4 TS/TSX files: OK. The extractor ran and `--check` is current.
- **NOT compiled, NOT type-checked, NOT run.** Things a first `cargo check` / `tsc` could flag: `let … else` use; `Context { .. ..Default::default() }` / `IntegrityData::default()` / `TraversalHints::default()` (all used identically in `run_spans.rs`); `blake3` (already a dependency); the `Query` extractor struct; `task_mgr.get_task(..).inputs` (`Option<Value>`); in the TSX, the narrowing of the const `overlay` inside the effect closure and the cast in `WalkGraph`.
- Unverified on real data: the container creation on the EU (first boot after deploy), `GetContainerContent` on the body, the live overlay against a real walk (`call_spans.jsonl` rows carry `trace_id`; confirmed by reading `instrumented.rs` and `trace.rs`, not by a run), the 3 mount points rendering. The `walk` graph is a static name-based reading and has never been compared with a real walk's executed sites.
- Known gap by design: a call still in flight is not in the live view (rows are written on finish); the overlay says so.
- Not done: nothing deployed, nothing written to the EU, CHECKLIST/STATUS untouched (lines in `CHECKLIST_LINES.md`).

## 5. Open decision — project code call graphs on workspace load

Smallest consistent design (not built): a ripple actor in `orchestrator/actors.rs` (same registry idiom as the existing four) that, on a `FileReference` write or a file-beacon change of a **Code** file inside a project, queues the file for the code modality pipeline through the SAME path `graphs.rs` uses in a walk (so jurisdiction/entry gates and capture apply), at `CallPriority::Loop`. Cost: one pipeline run per changed code file (no model call in the code modality — it is deterministic static analysis). Until then `CodeCallGraph` shows only what a walk created, and the cross-file resolution gap (pipeline 18 / E6) is unchanged.
