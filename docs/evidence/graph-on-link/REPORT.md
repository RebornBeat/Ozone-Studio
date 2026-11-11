# Graph on link — full graph creation on file link / change / workspace load (2026-11-11)

Operator rulings applied: file links ARE included; the FULL graph creation process runs without a prompt on file link, folder link, file change and
workspace load; every modality has a call-graph-like structure with entry points; no build, no test (rustfmt / `ts.transpileModule` parse checks only —
**uncompiled, never run**).

## 1. What the walk does for an attached file (traced)

`orchestrator/mod.rs` STEP 0 (`prompt_normalization`) → per attached file:
1. `detect_file_modality` (`ModalityType::from_path`, the shared `modality_ext` table; metadata-only buckets answer "unknown" → treated as text);
2. `read_to_string` of the WHOLE file (never cut / refused for size); non-UTF-8 falls back to `content_preview` (empty for a non-attached file);
3. `process_modality` → the modality pipeline's `Analyze` (code gets `code` + `language:null` + `file_path`; math gets `ParseExpression`);
4. the same pipeline's `CreateGraph` with `link_to_existing: true` → a ModalityGraph container (+ graph FILE `graphs/<modality>_<id>.json`);
5. later in the walk (`stages.rs:248`) `classify_file_graphs_post_creation` — an LLM comparing file-graph metadata with the PROMPT's intent (role relative to
   the prompt). Prompt-relative, so it has no meaning for a file linked with no prompt; it is not run here (see §6).

## 2. Where the events enter

| Event | Entry | Found at |
|---|---|---|
| file link | `file_link` pipeline `LinkFile` → `link_reference_to_graph` → `CreateContainer` (`FileReference`, name `File: <path>`) → GraphEventHub `created` | `assets/pipelines/general/file_link/main.rs:481`, `zsei/mod.rs:215-235` |
| folder link | `LinkFolder`: one `DirectoryReference` + one `FileReference` per DIRECT child file → each emits its own `created` | `file_link/main.rs` (LinkFolder arm) — covered file by file; nested sub-folders are not linked by the pipeline itself (a link-pipeline limit, unchanged) |
| file change | `file_beacon.rs` polls every FileReference's mtime/size (45 s) → `UpdateContainer` → `updated` event | `src/file_beacon.rs:73-117` |
| workspace load | nothing re-examined files at boot | — (new boot sweep, below) |

Existing consumer of those events: `CodeStalenessActor` (queues an AMT re-expansion *candidate* "call graph may be stale"). It stays; it never re-ran the
pipeline. `graph_on_link` now does.

## 3. What was built

`src/orchestrator/graph_on_link.rs` (new; `pub mod` in `orchestrator/mod.rs`; spawned in `lib.rs` beside the keyword actors).
* **Triggers → one queue**: GraphEventHub `FileReference` created/updated/linked; one **workspace-load sweep** (20 s after boot, over `file_beacon::discover_file_references`, now `pub(crate)`) — same code path.
* **One worker**, one job at a time, `CallPriority::Loop`, origin `graph-on-link`; **host-memory floor only** (`OZONE_MIN_AVAIL_MB`, default 1536): below it the worker WAITS (never drops). No cap on files / queue / size. Whole file read, hashed by streaming blake3.
* **Idempotent by content**: `<data dir>/graph_on_link_ledger.json` keyed by FileReference id → `{path, hash, modality, pipeline_id, status, trigger, task_id, graph_id, nodes, edges, entries, at}`. Same content + `complete` ⇒ skipped. A changed file ⇒ NEW task with `rerun_of` = the previous task (completed is final).
* **Work-order task per (file, content)**: kind `wo_graph`, key `file:<container_id>:<hash16>`, checklist `classify · analyze · graph · entries`, each ticked only by the real event (`TaskManager::create_work_task / complete_item`; completion derived by `settle`). A failed step records the error in the ledger, leaves the task OPEN, logs a warning; the next trigger for the same content resumes (a graph created earlier is not recreated; only `entries` is retried if the graph file was not readable yet).
* **Never fabricated**: the walk's `generate_id_static()` fallback for a missing `graph_id` is NOT copied — an absent id is a recorded failure.
* **Privacy**: reads FileReference + the file; announces nothing; no ui-ingest/McpEntity nodes involved; ledger + tasks local.
* **Switch**: `OZONE_GRAPH_ON_LINK`, ON unless `off`/`0`/`false`.
* `graphs.rs`: the modality-name → pipeline-id table became the free fn `pipeline_id_for_modality` (the walk's method delegates) — one table, two users. `span_out::read_graph_digest` and `file_beacon::discover_file_references` made `pub(crate)`.
* `GET /callgraph/files[?project_id=]` (`grpc/mod.rs`): ledger rows + each row's task status + `entry_concepts` for every modality; read-only, no prompt needed.
* UI: `components/FileGraphsView.tsx` (`fetchFileGraphs` in `ozoneClient.ts`): mounted in Engines → **Call graphs** (all modalities, modality chips, no prompt) and under the **Code** engine (`modality="code"`, compact). `WorkOrderTree` labels `wo_graph`. `wo_graph` tasks are excluded from the day view like every `wo_*` kind and read through `/order/work/<id>`.

**AMT**: no separate step. A CreateGraph write inside a project already flows `GraphEventHub → amt_loop::spawn_graph_ripple_sync → re-expansion candidate` (existing, unchanged), so the project AMT sees the new graph through the ripple exactly as it does for a walk-created one. **Keywords / validation**: the pipelines and the one container-creation door (`record_created`) do them on write, as in a walk.

## 4. Entry points per modality — produced today vs gaps

The rule is generic (`graph_fanout.rs`): explicit `analysis.entry_points` when the pipeline records them, else the **roots** (no inbound structural edge). Verified against each pipeline's `*NodeType` enum: all 26 modality pipelines with a binary have a root-type node (Image, Audio, Video, Sequence, Recording, Scene, Molecule, CADDocument, KinematicsSystem, ControlSystem, …), so roots are meaningful everywhere.

| Modality | Explicit entry points today | Entry (what the UI shows) |
|---|---|---|
| code | **yes** (`analysis.entry_points`, `call_sequence`, `Calls` edges; same-file resolution only) | functions with no caller (main, exports, handlers) |
| every other (25) | no — roots | the graph's root node(s) (scene/session/recording/document/…) |
| uidocument | n/a — library crate, no pipeline binary | none (no graph created; the table says so) |

Gaps (documented, not papered over):
1. **Only code records explicit entries.** A per-modality "primary entries" notion beyond the root (e.g. a CAD feature tree head, a control loop's controller) would be pipeline work in 25 crates — not done; roots serve until a modality asks for more.
2. **Code call graph is same-file only** (pipeline 18 / E6 cross-file resolution unchanged).
3. **Non-text files** (images, audio, video, 3D, binary data): the walk's attached-file step and this actor both feed the pipeline TEXT; a binary file fails `read_to_string` and is recorded `not_text_readable` (no graph, no task) instead of being analysed from empty text. A binary input path per modality is the real fix; it exists for neither the walk nor here.
4. **File role** (the prompt-relative LLM classification) is not run — there is no prompt. Modality/pipeline identification is the `classify` item.
5. **`FileReference → ModalityGraph` edge** is not written (the walk does not write one either); the ledger and `/callgraph/files` carry the mapping. A relation edge is a small follow-up through the same UpdateContainer path `file_link` uses.
6. `create_work_task` takes scope from a parent/other task; a `wo_graph` root has no parent, so its task scope is the default (user 0, no project) — project filtering of the UI list uses the ledger's `project_id`, not the task.
7. STEP 0 in the walk and this actor share the table and payload SHAPES but not the code (payload builders are copied, not shared); de-duplicating into one `build_file_graph` is a recommended follow-up once a compiler is available to check it.

## 5. Unverified (nothing was compiled or run)
rustfmt parses `graph_on_link.rs`, `grpc/mod.rs`, `lib.rs`; `ts.transpileModule` parses the three UI files. Unrun: the 4 new unit tests, the boot wiring, `mem_available_mb` visibility from the module (private fn at the crate root — visible to descendants), the real pipeline payloads for every modality except as copied from the walk, the UI against real data. First real check: link one text file and one code file, watch `/callgraph/files`, `/order/work/<task_id>` and the Call graphs tab.
