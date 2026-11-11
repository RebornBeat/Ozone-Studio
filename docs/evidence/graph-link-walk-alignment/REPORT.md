# Graph-on-link ↔ walk alignment (2026-11-11) — edit-only, parse-checked, NOT compiled/tested/deployed

Operator ruling: file link ALWAYS runs first and captures the non-semantic (structural / native) graph; the walk adds the intelligence on top; both know the other's current state (no duplicate graphs, no overwriting, no stale view).

## 1. Behaviour BEFORE this change (read from code)

| Question | Answer (file:line before the edit) |
|---|---|
| Did the walk reuse the link ledger's graph? | **No.** `graphs.rs::graph_attached_file` (STEP 0, `mod.rs:2309`) always ran `Analyze` + `CreateGraph` itself. It never read `graph_on_link_ledger.json`. |
| Did the link worker see a walk-made graph? | **No.** The ledger was written only by `graph_on_link.rs::process_job`. |
| Walk first, worker later | Two graphs for one content (two `CreateGraph` calls → two ModalityGraph containers; `link_to_existing` is the text pipeline's *related-container linking*, `text/main.rs:7404`, not "reuse an existing graph"). |
| Worker mid-run, walk starts | Same: parallel second graph, no coordination. |
| File changed since | Worker: new hash → new graph + new `wo_graph` task (`rerun_of`). Walk: always builds from the content it reads. The two could disagree on which content the graph describes. |
| Is the link graph "non-semantic"? | Only partly. The pipelines' native parse (text: chunk/paragraph/sentence/grammar nodes; code: static analysis; other modalities: their native reader) is structural. For text, `Analyze` is called with `extract_entities: true, extract_topics: true` (`file_reader.rs:82`), which are zero-shot LLM extractions (`text/main.rs:5026`, `:5037`) — the same flags the walk used. So today the link graph is NOT purely structural for text (see Options §5). |
| Does anything let a walk add semantic nodes to an existing graph? | **No.** `TextModalityAction::UpdateGraph` with `DeltaOperation::Insert` is a no-op stub (`text/main.rs:7447-7450`). So graph-level semantic layering is not possible without a pipeline change (Options §5). |

## 2. What changed

**One shared state, one shared lock** (`src/orchestrator/graph_on_link.rs`):
- Ledger row = the state both read: `hash`, `graph_id`, `structural_done`, `created_by` (`link`|`walk`), `task_id`, `previous_task_id`, `semantic_layers[]` (append-only: `{kind, by:"walk", at, detail}`), plus the existing status/trigger/nodes/edges/entries.
- `claim(store, path, hash, container_id)` takes a per-path async lock (`path_lock`), then reads the row. `existing_graph()` = the graph built for THIS content hash, only if its container still exists in the store.
- `claim_for_file` (walk entry): streams the blake3 of the file as it is now; returns `None` (walk proceeds exactly as before, records nothing) when unhashable / inline.
- `record_created` (walk built the graph): writes the row (`created_by: walk`, `structural_done`, `graph_id`, same `task_id` if same content, `previous_task_id` if the content changed).
- `record_semantic_layer(graph_id, kind, detail)`: appends to the row's `semantic_layers`; never touches structural fields.
- `put_row_key` is now one read-modify-write under `LEDGER_LOCK` and carries `semantic_layers`, `previous_task_id`, `created_by` over for the same content — a worker update never erases what the walk recorded.
- Rows the walk writes with no known FileReference id are keyed `path:<path>`; the worker adopts them into the container-id row (same content) on its first claim (`resolve_row`).

**Walk** (`graphs.rs`): `graph_attached_file` = claim → reuse if a graph exists for this content → otherwise the unchanged body (renamed `graph_attached_file_build`) → `record_created`. `classify_file_graphs_post_creation` appends the file role to `semantic_layers` for each classified file.

**Worker** (`graph_on_link.rs::process_job`): claims the path before reading the ledger (so it awaits a walk that is building the file), resumes from `existing_graph()` (skips analyze/create when the walk built it, with evidence text "built by the walk's attached-file step"), `rerun_of` also honours `previous_task_id` when the walk got there first.

**Outcome table (after)**

| Situation | Result |
|---|---|
| Link ran first (normal) | Walk finds `graph_id` for the content hash → reuses; its file role goes to `semantic_layers`. |
| Link job mid-run | Walk AWAITS the path lock (never fails open, never a second graph), then reuses. |
| Walk first / link not reached / switch off / earlier failure | Walk builds via its own pipeline calls, `record_created`; worker later skips analyze/create and completes the remaining items. |
| File changed since | Hash differs → nothing reused; new graph, new `wo_graph` task `rerun_of` the old one (completed is final). |
| Graph container deleted | `existing_graph()` is `None` (store check) → rebuilt, never a dangling id. |

**Surfaced:** `GET /callgraph/files` returns rows as stored, so `structural_done`, `created_by`, `semantic_layers`, `previous_task_id` appear with no handler change. `ozoneClient.ts FileGraphRow` and `FileGraphsView.tsx` show "structural graph by link|walk" and "walk layers: file_role = … (time)".

## 3. Text-graph-failure id minting — NOT changed (operator still deciding)

Exact places a failed text graph creation can still yield an id that is not a real container:
1. `src/orchestrator/graphs.rs:811-816` (`graph_attached_file_build`, TextLike|Math): `CreateGraph` error → `.unwrap_or_default()` (the error is swallowed, an empty `{}` result), then `graph_id_of(...).unwrap_or(Self::generate_id_static())` — a nanosecond timestamp becomes the file's graph id; `state.file_graphs` gets it (`mod.rs:2319-2322`) and AMT/seed code later treats it as real.
2. `src/orchestrator/graphs.rs:240-251` (prompt-modality graphs, `create_initial_modality_graphs`): same `unwrap_or(generate_id_static())` on a missing `graph_id`, after `Default::default()` on error (`:232-234`).
3. `assets/pipelines/modalities/text/main.rs:7414` (+ comment `:7387-7398`): when ZSEI persistence fails, `create_graph` returns success with the pipeline's in-memory local id.

Effect with this alignment: the minted id from (1) is written by `record_created` as `graph_id` with status `graph_created`; the next claim's `existing_graph()` checks the store and finds no such container → treated as absent (never reused), and the worker rebuilds. So the ledger cannot make a minted id "stick", but the walk still carries it for that run.

Options (consequences):
- **A. Fail loud (recommended):** return `Err` from both walk sites (and the pipeline's `success:true` on persist failure becomes an error). Consequence: the walk pauses on a store outage instead of proceeding with an unreachable graph (matches "failure pauses and resumes", "no silent {}"); a walk with an unwritable store stops earlier than it does today.
- **B. Mint nothing, continue without the graph:** `Ok(None)` like the binary-modality branch (graph absent, logged). Consequence: the walk proceeds, but loses file context for that run and may answer without the file — silent degradation unless surfaced in the response.
- **C. Leave as is.** Consequence: fake ids can enter `state.file_graphs`, AMT seeds (`stages.rs:3524-3530`) and the run checkpoint; failures hide until a later `GetGraph` misses.
- **D. A+B hybrid:** await/retry the store (it is the operator's own host), error only if the retry fails.

## 4. Unverified (parse-only: rustfmt parse + ts.transpileModule)
- Never compiled. Check at first `cargo check`: `Claim` holds an `OwnedMutexGuard` across `.await` (it is `Send`; the ledger `std::sync::Mutex` guard is never held across an await — `put_row_key`/`resolve_row`/`record_semantic_layer` are synchronous); `static LEDGER_LOCK: Mutex<()> = Mutex::new(())` (const since Rust 1.63); `claim(store, …)` shadows the `claim` fn with a local `claim` binding in `process_job` (rename if the compiler objects); `state.task_id` in `classify_file_graphs_post_creation`; `&*self.store` for `&dyn StoreAccess`.
- Never run: lock ordering walk↔worker, adoption of a `path:` row, `previous_task_id` → `rerun_of`.
- Residual race (documented, not closed): the walk reads the file's content (`mod.rs:2286`) separately from the hash taken in `claim_for_file`; a write between the two makes the recorded hash describe content one read earlier. The next trigger sees a new hash and rebuilds, so it self-heals.
- Different content for one path is locked by path, not by (path, hash); two simultaneous builds of different content for the same path serialize rather than race.
- Not done: the semantic layer is ledger state only. Graph-level semantic nodes need the text pipeline's `UpdateGraph Insert` implemented (stub today). The semantic file-role layer is the only walk contribution recorded.
- Text structural-only analysis (entities/topics off) was deliberately NOT done: with `extract_entities:false` the keyword fallback (`text/main.rs` "gated on extract_entities") is skipped, which can leave a text graph with no keywords (unsearchable). Option for the operator: an `Analyze` flag that keeps keywords but skips entities/topics, then the walk adds entities/topics as its layer.
