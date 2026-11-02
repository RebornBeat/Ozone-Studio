# Span-out, graph create/expand/use, and failure-pause/resume (2026-11-02)

All edits here are **uncompiled and untested** (operator-gated build + test pass). Companion docs: `SYSTEM_FLOW_AND_GRAPH_STATE_2026-11-02.md` (flow), `CALL_ROUTE_REGISTRY.md` (routes × guardrails), `CONTEXT_REGISTRY.md`, `GRAPH_RELATIONSHIP_REGISTRY.md`.

## 1. The doctrine corrections that drove this

1. **We never assign context to a step.** The AMT build, the modality graphs and the cross-modal passes already capture every relationship. What a step needs is what **spans out** from the things it is about, along those captured relationships. Fixed counts (4 seeds × depth 2 × 10 results × 8 containers) were a cap on context, the same mistake as a cap on responses; they are replaced by a **token budget**.
2. **Compaction is overflow management, not the gathering strategy.** The step-context compaction pass (LLM summarise, else a budgeted render) only runs when what spanned out exceeds the window share; it is downstream of span-out, never a substitute for it.
3. **We do not auto-continue past a failure.** A failed step **pauses the task**; it is resumed at any time from exactly that step.

## 2. Where graphs are CREATED vs EXPANDED vs USED

| Moment | CREATES a graph | EXPANDS a graph | USES the graph (span-out / read) |
|---|---|---|---|
| Stage 2 | attached-file graphs (per modality), prompt chunk graphs | — | — |
| Stage 3 | categories / methodologies (containers) | — | methodology + tool lookup by keywords |
| Stage 4b | one modality graph per verified modality | **cross-modal reference passes until 5 stable passes** (`RelatedTo` edges, CrossModalIndex); 15 modalities' `LinkToModality` | — |
| Stage 5 | the AMT container (+ `Continues` root relation across prompts) | AMT layers: intents → branches → details → cross-refs (+ methodology synthesis) | AMT traversal over graph-native sentence nodes |
| Stage 6–7 | Blueprint container | — | span-seeds: AMT, graphs, methodologies |
| **Every step start (new)** | — | — | **`span_out_for_step`**: seeds = attached-file / prompt-modality / chunk graphs, AMT, blueprint, methodologies + keyword rediscovery; best-first along `context.relationships` (type weight × confidence), structural children/parent; stops at the window share |
| Step execution | modality graphs via the step's pipeline (`CreateGraph` / `UpdateGraph`) | `graphs_updated` ⇒ `on_step_complete` reviews AMT alignment, may add branches/steps | tool context (flag-gated) |
| Stage 13 | response graph (text modality), ChatTurn | `link_walk_content`: `RelatedTo` between everything this walk made | — |
| After delivery | `ChatSession` / `ChatTurn` + response graph (background) | links turn → prompt graphs + response graph | next prompt's step context (last N turns) |
| Ripple (every write) | — | actors → findings → AMT **re-expansion candidates** | — |
| AMT re-expansion loop | — | deepens one unverified branch, persists in place, emits `AMTExpansion` | **span-out from the AMT + the triggering container** (new) |
| Tool outputs | `McpResult` / `McpEntity` graphs | edges from the tool's graph block | — |
| Ingest routes / farm | image + UiDocument graphs, `Renders` edge | — | — |
| File beacon | `FileReference` | version bump on change (ripples) | — |

The ripple is the connective tissue: **every** graph write emits through `ZSEI::query → graph_events::emit`, so creation and expansion alike wake the actors, the AMT ripple sync and the UI.

## 3. The span-out algorithm (`src/orchestrator/span_out.rs`)

* Seeds carry a score (1.0 = literally what the step is about; 0.5 = keyword rediscovery).
* Best-first expansion: pop the highest-scoring node, fetch, render, pack; then enqueue every captured relationship (`relation weight × edge confidence × node score`), structural children (0.85) and parent (0.6). Relation weights: `Continues` .95, `Contains` .9, `Renders/Supports/DerivedFrom/DependsOn/Calls` .85, `RelatedTo/SimilarTo/Locates/DepictsSameEntity/References` .8, unknown .6.
* Stops when the **token budget** is full, the frontier is empty, or the score floor (0.02) is reached. `OZONE_SPAN_OUT_MAX_FETCH` (default 400) is only a runaway guard on host reads; hitting it is reported (`FETCH CAP HIT` in the step's context provenance), never silent.
* Infrastructure containers (Root/User/Workspace/Project/Pipeline/…roots) are never rendered and never expanded through.
* Content: header + keywords always; the container's content-file text (`text/content/summary/response/prompt/description/reflection/note` values) and file summaries are included **whole or not at all** (never cut mid-text).
* Used by: step startup (`stages.rs` → `span_out_for_step` → `context_aggregation` `span_out_context`), the AMT re-expansion loop (`amt_loop.rs`, window share of the answering model), and the I-Loop reflection (`i_loop.rs`: spans out from the signalled containers, a third of the reflecting model's input budget). Ripple reactions use it through that loop (an actor only records a finding; the deepening that follows spans out from the triggering container).
* `context_aggregation` change: it no longer chops the assembled string at the budget; the keyword-found containers get the *remaining* budget, host-supplied layers pass whole, and overflow is reported for the caller's compaction pass.
* Tests added (unrun): spans past depth 2 along relationships, stops at budget, infrastructure never rendered.

## 4. Failure doctrine — pause, never continue; resume from exactly there

* **Where a failure pauses:** a failed step (including a failed sub-step, a web-search step whose every sub-query failed, and a run that ends with no response) → `PromptOrchestrator::pause_on_step_error` → checkpoint written with `paused_at_step` + error → `TaskManager::pause_for_error` (status `paused`, error `paused at step N: …`, the step row `failed`) → the walk stops with `PAUSED: task T paused at step N: … It was not continued…`. `/orchestrate` answers `status:"paused"`.
* **Checkpoint:** `<data>/runs/<task_id>.json` (`orchestrator/run_checkpoint.rs`), written after task creation and after every completed step (write-then-rename): request, cleaned prompt, keywords/topics, methodology + category ids, graph ids, blueprint steps, applicable tools, attached-file contents, completed step outputs. Not restored (documented): the in-memory AMT tree object and intent/branch capture lists.
* **Resume:** `POST /task/resume_run {task_id}` flips `paused → running` immediately, then runs detached: rebuild state for the saved request, overlay the checkpoint, re-enter the step loop (completed steps skipped), then result collection → post-execution → delivery (`execute_resumed_stages`). `POST /task/pause_info {task_id}` says whether a task is failure-paused, at which step, and whether it is resumable. A second Resume is refused. A task without a checkpoint is refused with a pointer to `/task/step/rerun`.
* **Where it shows:** `/order/global` entries carry `error` + `paused_for_failure` (the *paused* group); OrderPanel shows a **Resume** button on those rows; TaskDetailPanel shows a banner with the error and **Resume from step N**; the meta chat and every project chat show a `PausedBanner` with **Resume** (shared `useChatSession.paused/resumePaused`); status shows in the status bar/order like any task.
* **Boot:** failure-paused tasks stay `paused` across a restart (only queued/running/awaiting are reconciled to `interrupted`) and remain resumable from the checkpoint file.
* **Not paused (different doctrines):** meta/consciousness gates that cannot judge **await** (durable, meta-only); jurisdiction confirmations are held for the operator; cancel is a user action and is never overridden.

## 5. Consciousness pipelines — linked by name, honestly

`pipeline::registry::binary_alias` maps a registered folder name to the crate that actually exists. Extended 2026-11-02 after checking each crate's action vocabulary and doc header:

| id | registered | crate / binary | evidence |
|---|---|---|---|
| 39 | consciousness_decision_gate | decision_gate | (already) — and the walk uses `DecisionReviewExecutor` instead |
| 40, 41 | experience_categorization, core_memory_formation | experience_memory | doc: "Pipelines #40-42 (Categorization, Core Memory, Retrieval)" |
| 42 | experience_retrieval | experience_search | (already) |
| 43 | emotional_baseline_update | emotional_state | has `ProcessTrigger` (the stage-12 call) |
| 44 | i_loop | reflection | doc: "(I-Loop)" |
| 45, 46, 53 | internal_language, narrative_construction, voice_identity | self_model | has `GetVoice`; doc: "Internal Language, Narrative Construction" |
| 47 | relationship_development | relationship | has `RecordInteraction` |
| 50 | playback_review | experience_playback | doc: #45 playback |
| 51 | user_feedback | emotional_response | doc: "User Feedback Integration" |
| 52 | collective_consciousness | collective_consciousness | same name |
| 54 | meta_portion_consciousness | consciousness_dashboard | dashboard crate |
| 48, 49 | ethical_assessment, ethical_simulation | — | no crate |

Caveats: none of these has a built binary in the shared target dir (`scripts/build-pipelines.sh` builds only those already present; add `OZONE_BUILD_NEW="…"` for a first build). Linking makes them *findable*; several crates' action vocabularies differ from what the orchestrator sends (e.g. no `StoreExperience`, no dashboard `Update`), which is why stage 12 now verifies the host store's records and stage 13 no longer dispatches the dashboard call. Which of these the walk **actually uses today**: pipeline 39 (intercepted by `DecisionReviewExecutor`, not the crate), 47 (relationship, stage 12, still dispatched), 46/`GetVoice` (stage 13 voice identity) — everything else is served by the host-native consciousness store.
