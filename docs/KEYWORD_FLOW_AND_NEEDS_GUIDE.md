# Keyword flow and needs — creation, retrieval, hits, origin, and training preparation (2026-11-08)

Operator ask (2026-11-08): *"ensure we capture the full keyword creation flow … when they are created so they actually work as supposed to when retrieved … identify all keyword needs … a guide of all needs for all keyword searches, captured in our algo docs … hits when we create methodologies — its first hits: the files its workspace is handling, from where it was called, project scoped or global … capture properly in ALL states of keyword creation and retrieval and have everything prepared for training."*
This guide is the single place that answers: **what must be recorded at each state, where the code records it, and what is still a gap.** Companion docs: `KEYWORD_CAPTURE_AND_ML_GUIDE.md` (design, model, rulings), `LITERAL_REMOVAL_2026-11-08.md` (hotness / weight / confidence), `CALL_ROUTE_REGISTRY.md` R21–R27, `STATIC_LOGIC_REGISTRY.md` (search strategies, rank key), `CONTRACTS.md` (shared `keyword_model.rs`), `ZERO_SHOT_CALL_REGISTRY.md` §19.

## 1. The five states of a keyword, and the rule for each
| State | What happens | Where (code) | The rule |
|---|---|---|---|
| **CREATE** | a container is born with keywords/topics/name | the ONE write door `zsei/query.rs::create_container` (every host writer and every pipeline's `/zsei/query` CreateContainer passes it) | every creation leaves a **`created` event** carrying the container's type, name, initial keywords/topics, typed scope, parent, provenance and the **ORIGIN** (§2). Nothing is created invisibly. |
| **ENRICH** | the keywords are corrected and expanded | `keyword_settle.rs` (Pass A verbatim → B expansion → C unanimous-drop keeper → D re-judge) + the generic graph-identifier extractor | every keyword carries provenance (`keyword_meta[term].source`); every keyword change leaves a **`keywords_changed` event** (added / removed, by whom). |
| **RETRIEVE** | a search finds containers | the ONE search door `zsei/search.rs::SearchRegistry::run` (+ `StoreAccess::search_ranked`) | every search leaves a **`search` event**: query terms, strategy, returned ids, **ORIGIN**. |
| **OUTCOME** | what happened to a retrieved container | span-out (`packed`), Stage 3 (`gathered`, `kept`/`dropped`), rule gate, blueprint (`applied`), review (`human`) | outcomes are events with the SAME trace id as the search, so a search joins to its outcomes. |
| **TRAIN** | the model learns from the joined history | `keyword_program::build_dataset` → pipeline #17 `TrainModel` | only REAL joined events become samples; held-out groups are the last in time; promotion is a measured yes/no. |

## 2. ORIGIN — "from where it was called" (workspace, project, global, files)
Every event carries an **origin** record. The same record is attached to creation, search and outcome events so scope and provenance are never reconstructed afterwards:
```
origin = { call_site, trace_id, task_id?, workspace_id?, project_id?, global, request_terms[], file_ids[] }
```
* `call_site` — which door opened this: `orchestrate` (a walk), `meta-loop` / `amt-loop` / `i-loop` / `assistant` (background actors), `http:<route>`, or the **pipeline name** (`text`, `code`, `methodology_create`, `context_aggregation`, …).
* **project scoped vs global** — `project_id`/`workspace_id` come from the request that started the work (`OrchestrationState.request`), or for a pipeline from the `ExecutionContext` the host passed it; `global = true` when neither is set. The container's own **typed scope** (`Context.scope`, derived centrally from the `ws:` / `proj:` / `scope:global` markers) is recorded beside the origin so a creation inside a project but stored globally (or vice versa) is visible.
* `file_ids` — the files the scope is handling at that moment: the attached-file graphs of the walk (`state.file_graphs`, grown as Stage 4 graphs them) — for a methodology created during a walk these are its **first-hit files**.
* `request_terms` — the terms of the request that triggered the work (never the raw text; a hash of the request is stored for dedupe).
**How the origin propagates:** orchestrator walks and loops install a task-local origin (`keyword_program::with_origin`, like the trace). A pipeline is a separate process: the executor stamps `OZONE_ORIGIN` (JSON of call_site, workspace, project, task) beside `OZONE_TRACE_ID`, and every pipeline's `/zsei/query` call now sends `x-ozone-trace` and `x-ozone-origin` headers (33 reqwest pipelines + `depth`'s raw HTTP patched 2026-11-08); the `/zsei/query` route reinstalls the trace and origin around the query. Without this a pipeline-originated search had NO trace and could never be joined to an outcome label.

## 3. CREATION flows — who creates containers with keywords, and what is captured
| Flow | Who / where | Keywords assigned from | Origin available | Captured now | Gap |
|---|---|---|---|---|---|
| **Methodology — runtime/closure draft** | `stages.rs::draft_methodology_for_capability` → pipeline 12 | the drafted JSON (`keywords`, `topics`) | walk: request, project/workspace, `file_graphs` | `created` (central) + origin (task-local) + `gathered`/`applied` later | the originating request terms are the **first hit** (request → methodology) |
| **Methodology — meta-loop draft/modify** | `meta_loop.rs` (gap drafting, `modify_methodology_for_gap`, quality audit) | gap keywords/topics | the gap's recorded request + scope | `created` / `keywords_changed` (central) | gap rows carry no project/file context yet (the review queue stores request + keywords only) |
| **Methodology — base/seed/bootstrap** | `mod.rs` runtime base, `lib.rs` seeds, `methodologies/store.rs` | static lists | none (global) | `created` with `global = true` | fine — global by nature |
| **Methodology — marketplace/MCP import** | `mcp_graph.rs`, marketplace routes | provider-declared | the HTTP caller | `created` (central) | provider identity not in origin |
| **Blueprint** | `stages.rs` blueprint store, `blueprint_create` pipeline | model-drafted + methodology keywords | walk | `created` + origin | the originating request terms should be stored as the blueprint's own request signature |
| **Modality graph (27 modalities)** | each modality pipeline `CreateGraph` via `/zsei/query` | node identifiers / labels / extraction | pipeline ExecutionContext (project/workspace) | `created` with pipeline call_site (headers) | creation-time identifiers still depend on the pipeline; the generic graph-file extractor back-fills (Pass A) |
| **File reference / file graph** | `file_link`, `url_link`, `package_link`, `workspace_tab` | name, extension, analysis | project file attach | `created` + origin | directory tokens not captured |
| **Chat session / turn** | `chat_store.rs` | prompt/response terms + scope markers | request scope | `created` + typed scope | — |
| **Experience / consciousness** | `consciousness/store.rs`, `review.rs` | task/outcome terms | global | `created` | scope is global by design |
| **Coordination events / run spans / loops** | `context_mirror.rs`, `run_spans.rs`, `actors.rs`, `i_loop.rs` | markers (+ few terms) | loop trace | `created` | 72 % of their slots are markers — meaning terms are sparse by nature |
| **Project / Workspace** | `workspace_tab` pipeline | name/description | the user action | `created` | no keywords until the settlement roll-up folds the children |
| **Tool / MCP entity, MCP result** | `mcp_graph.rs` | tool name, capabilities, `call:*` markers | the MCP call context | `created` | capability sentences need aliases (settlement Pass B) |
| **Model containers (keyword models)** | `keyword_program::register_model_containers` | `keyword-model`, kind, id | meta loop | `created` | — |

### 3.1 The first hit of a methodology (what the operator asked for)
When a methodology is created, three facts are the **first hits** that make it findable and trainable, and all three must be on record at creation:
1. **Why it was created** — the originating request's terms (the `created` event's `terms`, from the origin), the gap's keywords, and the trace id (so the walk that wanted it is joinable).
2. **Where it was created** — `workspace_id` / `project_id` / `global`, plus the typed scope the container ended up with; this is what lets a project-scoped request find project methodologies without leaking other projects'.
3. **What the scope was handling** — `file_ids`: the attached/graphed files at creation. They tie a methodology to the files that motivated it (a "static analysis" methodology made while a Rust project was open is retrievable through those files' graphs).
In the dataset these become a **creation pair** (request terms → the created methodology = a positive; the methodologies gathered in the same trace and not chosen = negatives), so the encoder learns the request vocabulary → methodology mapping from the moment of birth, before any retrieval history exists.

## 4. RETRIEVAL — every keyword search and what it needs
| Search site | Query terms from | Container types | Scope it needs | Strategy | Outcome label available | Logged |
|---|---|---|---|---|---|---|
| Stage 3 methodology gather (`mod.rs`) | sanitized request keywords | Methodology | project + global methodologies | default `scan` (recall-sensitive; judged afterwards by E13) | `kept`/`dropped` (E13), `applied` | search ✓ |
| Blueprint match (`stages.rs` ×3) | request keywords | Blueprint | project + global | default | reused / not | search ✓ |
| AMT hop seeds (`amt.rs`) | AMT keywords | any | request scope | `search_ranked` | `packed` | ✓ |
| Prompt cross-link (`graphs.rs`) | prompt terms | any | request scope | `search_ranked` | edges written | ✓ |
| Finding-key lookup (`actors.rs`) | `finding:…` marker | any | global | `search_ranked` (marker exact) | — | ✓ |
| Step span-out seeds (`span_out.rs`) | branch keywords + step content words | any | **project/workspace-scoped** (+ global knowledge) | `search_ranked` + scope filter | `packed` | ✓ |
| `/context/span_out` | caller keywords | any | optional project/workspace | `search_ranked` | — | ✓ |
| Step context seeds (`context_aggregation` pipeline) | prompt/step words | any | host-wide today (gap) | `token` | `packed` via span-out | ✓ (trace via header) |
| Graph linking (text/code/math `link_related_containers`) | graph keywords/topics | any | project + neighbourhood | `token` | edges written | ✓ (trace via header) |
| Jurisdiction rule lookup | region/scope codes | JurisdictionRuleSet | global | `exact` | — | ✓ |
| Chat session lookup (`chat_store.rs`) | `chatsession:…` marker | ChatSession | scope marker | `exact` | — | ✓ |
| Meta-loop gap coverage (`meta_loop.rs`) | gap keywords | Methodology | global | default | handled / drafted | ✓ |
| Voice, task categories | fixed keys | various | — | default | — | ✓ |
**Needs common to every keyword search** (the checklist a new search must satisfy): (1) query terms passed WITHOUT a word list — shape rules only; (2) the intended **scope** stated (project, workspace, global) and enforced; (3) the container **type** filter if known; (4) a **strategy** chosen on purpose (`exact` for ids/markers/codes, `token` for flood-prone seeding, `scan` only where recall beats precision); (5) results consumed **in rank order** under a budget (never an arbitrary slice); (6) the **outcome** of using a result recorded under the same trace; (7) an **origin** on the event.

## 5. Hits and hotness
A **hit** is a (query term → container) match event; **hotness** is the OBSERVED ratio packed ÷ returned per container, derived from hits by the rollup, mirrored to `hints.hotness_score`, and READ by the ranking (`RankFacts.hotness`). The invented per-node hotness literals are removed (`LITERAL_REMOVAL_2026-11-08.md`). Hits therefore feed (a) ranking directly through hotness and the counts, (b) the labelled dataset, (c) keyword validity (a term whose hits are net-dropped is re-judged).

## 6. Training preparation — what a sample is, and that nothing is lost
* **Ranking sample** = one `search` event + the outcome events sharing its trace: query terms, candidate container terms, a facts snapshot from BEFORE the query (counts, hotness), label (applied/kept/human-yes positive; dropped/human-no/gathered-only negative). Time-ordered; the last 20 % held out.
* **Creation sample** = one `created` event (request terms from the origin → the created container) with the same-trace `gathered` containers as negatives.
* **Judgment sample** = every strength-N vote (`judgment_samples.jsonl`: kind, subject, request, question, yes/no, strength).
* **Persistence:** the logs are append-only, never rotated or truncated; the rollup and the dataset builder stream them.
* **Provenance of every sample:** trace id, origin (call_site, scope, files), strategy, and the model version that ordered the candidates.
* **What is never stored:** raw request text (terms + hash only) and any model-reported confidence.

## 7. Verification checklist (real host)
1. Create a methodology through a walk inside a project → the hit log has a `created` event with that project's id, the walk's file ids and the request terms. 2. Create the same kind globally → `global: true`, no project. 3. Run a pipeline-originated search (text linking) → its `search` event carries the walk's trace id (header forwarding). 4. A span-out step → `packed` events with `via`; `edge_use_stats.json` grows. 5. Edit a container's keywords through the update door → a `keywords_changed` event with added/removed terms. 6. `build_dataset` → groups include creation pairs; `keyword_training_set.jsonl` loads in pipeline 17 `TrainModel`.
