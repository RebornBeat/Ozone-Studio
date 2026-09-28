> Doctrine: OZONE_STUDIO_SPECIFICATION.md §6 (ZSEI) + §7 (Context Storage).
> Core law: **"ZSEI stores context, not copies."** Everything linked becomes
> a graph node with semantic meaning, relationships, and integrity. The
> graph is the system's memory; traversal — not search-and-concatenate —
> is how context is gathered (§6.7).
>
> This document is the honest status of that doctrine versus the live
> implementation. Updated additively; stale rows get fixed, not deleted.

Last verified: 2026-09-15 (ZCode + Claude Code joint review night)
Re-verified: 2026-09-21/22 (claude-code, real living-graph test — see delta section below; rows below corrected in place per this doc's own additive convention, not deleted)

---

## The doctrine (spec → implementation contract)

| Spec principle | Meaning |
| -------------- | ------- |
| Structure before intelligence | Graph containers before any model reasoning |
| Context, not copies | Link files/URLs/packages; store semantic meaning + relationships |
| Traversal before generation | Context is gathered by walking the graph (§6.7 modes), never dumped as flat text |
| Everything indexed | ZSEI containers under structural roots; GlobalState = parent/child; LocalState = metadata/context/relationships |

## Status matrix — what is graphed today

| Surface | Containers | Relationships | Served to context via | Status |
| ------- | ---------- | ------------- | -------------------- | ------ |
| Text modality graphs | real for attached files (2026-09-21/22 live: containers 30425/30427/30429/30431/30434) — but the attachment path (`create_graph` in text pipeline) only ever emits `Document`/`Section`/`Entity`/`Topic`/`Keyword` nodes; **no `Sentence` nodes, no grammar/verb graph** (`GrammarNodeType::{Sentence,Verb,MainVerb,...}` exists in the same file but is built by a *different* code path — the prompt's own sentence-chunk extraction for AMT building, not attached-file CreateGraph). `Entity` came back empty for our test content (real LLM call, `extract_entities_from_text`, pipeline 9 — correctly empty: generic gradient-descent prose has no PERSON/ORG/LOCATION/DATE/etc, not a bug). | real, live-verified `SimilarTo` edges between sibling text graphs and cross-modal to code (see below) | modality containers, cross-process retrieval **BROKEN** (see gaps) | ⚠️ partial — richer than one line captures, see delta section |
| Code modality graphs | real (AST via tree-sitter → `File`→`Function` containers, live-verified 2026-09-21/22: containers 30426/30428/30432). **Gap found live**: `function_calls: []` and every function's own `calls: []` come back empty even when the real source has real calls (`gradient_descent` calls `gradient()`, `main` calls `gradient_descent()`) — the call-graph/dependency-edge schema exists (`CodeAnalysisResult.function_calls`, `FunctionInfo.calls`) but isn't populated by the tree-sitter query used today. No function-to-function edges exist in the graph as a result — only `File --Contains--> Function`. | relationships array present and **live-verified real** (see below) | same cross-process gap as text | ⚠️ partial — structural capture real, call-graph edges missing |
| Attached-file auto-routing (stage 2 / mod.rs STEP 0) | real — `detect_file_modality` (~60 extensions → ~25 modalities), full-content Analyze + CreateGraph per attachment, every request, nothing silently dropped (fallback = text/100) | **FIXED, live-verified 2026-09-21/22** (was the "MISSING" row below — now corrected): `link_to_existing: true` at the mod.rs STEP 0 call site, real `SimilarTo` edges confirmed on disk across two independent full runs — text↔text (0.9 near-duplicate / 0.75 partial), text↔code (0.45 partial-overlap), code↔code (0.9 near-duplicate), reaching even unrelated prior-session containers sharing ≥2 keywords. Per-request file-role classification (primary/supplementary/raw) is **still broken** — separate finding: the classification LLM call's own `"response"` field comes back empty (`raw=""`, real non-zero `tokens_used`) so `classified_file_graphs` stays empty every time; not root-caused yet. | modality containers per attachment, real cross-relationship edges | ✅ cross-relationships (task 57 closed for text/code) — file-role classification still ❌ |
| Math modality graphs | real persisted graphs as of 2026-09-21/22 (two bugs fixed this session: (1) math has no `Analyze` action, `ParseExpression` used instead; (2) math's `CreateGraph` needs a field literally named `analysis` typed `MathAnalysisResult`, not the generic `analysis_result` blob text/code use, plus `graph_id` nests under `result` unlike code's top-level field — both call sites in mod.rs and graphs.rs fixed). Live-verified: container 30433, 20 nodes/19 edges, real structure, no longer silently failing. | **Zero relationships — confirmed isolated, root-caused.** `ParseExpression` is built to parse ONE clean mathematical expression, not a whole document. Fed a real `.tex` file (prose paragraphs + LaTeX), it character-tokenized the prose and extracted the file's distinct alphabetic characters as spurious single-letter `Variable` nodes (`d`,`o`,`c`,`u`,`m`,`e`,`n`,`t`,...,`type: Unknown`) instead of the real terms (`theta`,`learning_rate`,`epsilon`). `derive_math_keywords` (math/main.rs:84) only reads `Variable`/`Axiom`/`Theorem`/`Definition` node labels — with garbage labels in, it produces zero meaningful overlap against sibling text/code graphs' real keywords, so `link_related_containers`'s ≥2-shared-term threshold correctly finds nothing. Not a crash — a real architecture gap: math has no equivalent of text's entity/topic extraction for mixed prose+math content. `LinkToModality` exists in `MathAction` but requires the caller to already know both graph ids — it's an explicit-link primitive, not a discovery mechanism, so it doesn't fill this gap as-is. | works (in-process); cross-process via CLI same open gap | ✅ in-process graph creation / ❌ real cross-modal linkage (architecture gap, not fixed this pass — see delta section for proposed directions) |
| File links (file_link pipeline) | references + analysis JSON under `local/file_analysis` — **not yet ZSEI containers with edges** (ZSEIQuery::LinkFile exists, unwired here) | stored as project references | project file lists | ⚠️ flat |
| URL / package links | same pattern as file_link | same | same | ⚠️ flat |
| Workspaces / Projects | real containers with parenting (live-verified: Workspace 1081 → Project 1082; GetProjects works) | parent/child only — no cross-links yet | task creation, ForStep(project_id) | ✅ core / ⚠️ thin edges |
| Jurisdiction (27 scopes) | real containers under JurisdictionRoot — **flat siblings, no meta-workspace graph, no relationship edges** | none | flat keyword-scan + text dump (the SAST-style failure mode) | ❌ needs graphing (task 56) |
| Methodologies | real containers + real content (3 of 16 with deep content; index of 16 incl. Host Lifecycle) | category links one-directional | methodology search (relevance-fixed) | ✅ / ⚠️ category edges |
| Blueprints | containers registered; content stranded (assets/blueprints missing + old schema) | — | 100%-match reuse path unreachable | ❌ stranded |
| **Coordination graph (NEW, task 42)** | CoordinationEvent containers under /SharedContext root (id 8) — notes/decisions/handoffs/claims, scoped global/ws:/proj: | keywords (kind/agent/file:), claim dedupe; relationship edges v2 | monitor feed today; AMT layer = task 43 | ✅ fresh, edges thin |
| **Context objects (NEW)** | per-step assembled context persisted on the task record (`context_assembled` + `context_sources`) | task-record linkage | GET /task/get steps | ✅ fresh |

## The gaps (root-cause level — all confirmed live)

0. **Nested pipeline-9 extraction calls fail silently — no fallback walk**
   (SETTLED 2026-09-16, low-contention re-test): attached-file graph
   creation works (auto-route ✓, containers created ✓), but the nested
   topic/keyword extraction calls inside text modality's create_graph
   failed for BOTH attachments (empty keywords/topics) while the main
   generation walked the fallback chain successfully (6 distinct models
   observed in one request). Root cause: SubprocessExecutor's direct
   pipeline-9 call doesn't route through
   `walk_fallback_chain_standalone` the way main steps do — one
   free-tier/rate-limit failure = silent empty extraction. FIX: route
   nested extraction calls through the same fallback walk (the shared
   logic exists; wire it). Additionally: extraction failures should
   downgrade gracefully (graph persists with whatever WAS extracted —
   file text itself — rather than empty context).

1. **STALE, corrected 2026-09-22 — the TraversalEngine is no longer dead
   code.** Original claim (task 56, claude-code, this row's history kept
   for context): `src/zsei/traversal.rs` implements §6.7 for real —
   Structural / Semantic / Contextual / Hybrid / MLGuided / BruteForce —
   with zero call sites in any context-assembly path, everything (including
   jurisdiction) using flat `SearchContainersByKeywords` instead. **This is
   no longer true.** Confirmed live by direct call-chain trace (not
   assumed): `assets/pipelines/general/context_aggregation/main.rs`'s
   `traverse_from_seeds` (used on every step of every live request, stages
   9-11) issues a real `ZSEIQuery::Traverse` request, which `src/zsei/
   query.rs:129-131` dispatches to `traversal.traverse(...)` —
   `TraversalMode::Structural` routes to the literal `TraversalEngine::
   structural_traversal` (`src/zsei/traversal.rs:48/154`) this row
   originally called dead. Per that pipeline's own comment, the fix landed
   the same night it's dated ("fixed earlier tonight") and specifically
   follows both parent/child structure AND real `Relation` edges — meaning
   it now genuinely benefits from the text/code modality `SimilarTo`-edge
   work also done this session (a traversal from one matched container can
   reach a second, genuinely related one that flat keyword search alone
   would miss). **UPDATE 2026-09-22 — this remaining gap is now CLOSED,
   verified by direct read, not assumed.** AMT building and blueprint
   assignment DO now receive relationship context — not via `Traverse`
   itself, but via the cheaper direct-fetch fix this row already
   recommended (`state.file_graphs`'s ids are known exactly, no discovery
   needed). Confirmed real: `jurisdiction_summary` (`stages.rs:135`) and
   `file_relationship_summary` (`stages.rs:208`) are genuine functions,
   both called at `amt.rs:460-461` (`build_amt_from_graphs`),
   `amt.rs:1152-1153` (`build_amt_layer_by_layer` — the path every live
   request this session actually used), and `stages.rs:391-392`
   (`stage_3_blueprint_assignment`). The "what work to do" stages are no
   longer relationship-blind. See `docs/ZERO_SHOT_CALL_REGISTRY.md` §11-12
   and `docs/CONTEXT_REGISTRY.md` §2 for the full wiring matrix.
2. **`step_contexts` was write-only** (fixed tonight): assembled per-step
   context was never read, persisted, or exposed. NOW: Stage 7 persists
   every step's context object onto the task record
   (`update_step_context` → `context_assembled` + `context_sources`
   provenance, visible in `GET /task/get`). **Update 2026-09-22, since gap
   1 above is now partially closed**: confirmed `context_aggregation/
   main.rs` genuinely emits two distinct provenance labels, not one —
   `"keyword-scan"` (line ~448, the flat search) and `"traversal"` (line
   ~455, containers actually discovered via `traverse_from_seeds`' real
   `Traverse` call). Not the exact `traversal:<mode>` granularity this row
   originally predicted (no mode suffix), but the underlying prediction —
   that the provenance would stop being uniformly `keyword-scan` once real
   traversal existed — is confirmed true.
3. **Cross-process graph retrieval** for text/code modality (claude-code
   finding): in-process caches don't survive the CLI boundary — math
   modality's fix is the reference; text/code pending.
4. **Jurisdiction is flat** (claude-code finding, user directive): region
   containers are siblings, not a meta-workspace graph (UN → regional →
   national hierarchy with relationship edges), so jurisdiction context
   can't be traversed or looped like AMT branches.
   **UPDATE 2026-09-22 (GAP-C1, `CONTEXT_REGISTRY.md` §3)**: a real,
   separate fix landed for a related but distinct problem — content↔
   jurisdiction edge-mixing (why a text/code graph never linked to the
   rule that governs it). `src/lib.rs:559-599` (inside `AppRuntime::new()`,
   confirmed live/reachable at boot) now derives real content keywords
   from each rule's own `condition` text (tokenized, ≥4 chars, deduped,
   capped at 12) instead of the old `[scope, "jurisdiction"]`-only
   keywords — genuine, substantive logic, not a comment. **But unproven on
   real data**: checked all 68 real `JurisdictionRuleSet` containers on
   disk directly — every one still has the old thin 2-keyword shape,
   because no scope has been freshly registered since this landed (every
   recent boot log shows "0 new registrations, 41 scopes already
   registered"). The fix is real and forward-looking, not retroactive —
   existing containers won't gain rich keywords until either a new scope
   registers or a re-registration/backfill path is run. This does NOT
   touch the flat-siblings/no-meta-workspace-graph problem this gap
   entry is actually about — that remains open.

## The path (active, routed through the host task system)

| Task | Owner | Closes |
| ---- | ----- | ------ |
| 56 | claude-code (lead) + zcode | TraversalEngine wired into context assembly; jurisdiction meta-workspace graph (first concrete case) — **relationship edges LIVE (40 wired at boot 18:41)**, live traversal verification pending |
| 57 | claude-code | Attached-file graphs cross-linked into the whole graph (link_to_existing implemented, relevance = scope-first + keyword overlap + DiscoveryMethod provenance + role-labeled edges) — **CLOSED for text/code, live-verified 2026-09-21/22**; math produces real graphs but doesn't cross-link yet (architecture gap, see delta section); file-role classification (primary/supplementary/raw) still broken separately |
| 43 | zcode | Coordination graph → AMT stage context as its own scoped layer (global + ws:+proj: keywords filter; separate-layer doctrine) |
| 45 | zcode | Orchestrator stages issue McpCall for tool needs |
| 44/46 | claude-code | CHECKLIST retirement (post context-transfer); host-ops verification |

Sequencing rule (methodology 16): restarts are batched — implementation
lands build-first, restarts happen when the operator pulls the trigger on a
batched `[host-ops]` request.

## Context provenance contract (new, 2026-09-15)

Every step's context is now a **captured fact**, not an assumption:
`GET /task/get` → `steps[].context_assembled` (the exact text the step
received) + `steps[].context_sources` (mechanism: `keyword-scan` today,
`traversal:<mode>` after task 56). No fabricated context claims anywhere —
if a step's context wasn't captured, the field is absent, not guessed.

## 2026-09-21/22 delta — AMT ripple + merge-back verified real; math's isolation root-caused; a proposal, not yet built

Grounded in the real 3-file (text/code/math) living-graph test this session (see CHECKLIST.md "living-graph test re-run" entries). All findings below are read directly from source + live disk data, not inferred.

**AMT ripple confirmed universal, not per-modality.** `src/orchestrator/amt_loop.rs::spawn_graph_ripple_sync` subscribes to `crate::graph_events::GraphEventHub::global()`, a broadcast channel fed from exactly one choke point: `ZSEI::query` (`src/zsei/mod.rs:129-171`, doc comment literally calls itself "THE graph write choke point"). Every `ZSEIQuery::{CreateContainer,UpdateContainer,DeleteContainer,LinkContainers}` mutation — regardless of which modality pipeline issued it — emits a `GraphEvent` there. This means mod.rs STEP 0's per-file `CreateGraph` calls for text, code, AND (now that it's fixed) math all genuinely ripple into AMT re-expansion candidates the same way; this was previously unverified, now confirmed by reading the choke point directly, not assumed from the architecture doc's description.

**Merge-back is real and already landed — `docs/AMT_EXPANSION.md`'s "v2, next" label for this is stale.** `PromptOrchestrator::merge_back_to_main` (`src/orchestrator/amt.rs:284-360`) is called from `persist_amt_container` whenever a project-scoped AMT is persisted (`amt.rs:274-276`). Real logic: finds the project's main AMT file (earliest `amt_p{project_id}_*.json` by embedded millisecond timestamp), content-string-dedups the fork's verified top-level children against main's existing content, grafts genuinely new ones, writes the file. Covered by a real unit test (`amt_merge_back_grafts_fork_branches_into_project_main`, `amt.rs:3101`). **One real, minor gap found while verifying this**: the graft's own write is a raw `std::fs::write` (`amt.rs:358`), not a `ZSEIQuery` mutation — it does not go through the `ZSEI::query` choke point above, so a merge-back graft does **not** itself emit a ripple event. Low-severity (the fork AMT's own creation already rippled once), but worth knowing: a graft-only change to the main tree is silent to any ripple subscriber (websocket, monitor, this same AMT loop).

**Math's isolation — root cause confirmed, three fix directions evaluated, none built yet (real design decision, not mechanical):**
Root cause (see status matrix row above): `derive_math_keywords` (math/main.rs:84) only ever reads labels off `Variable`/`Axiom`/`Theorem`/`Definition` graph nodes, which for a prose document come from `ParseExpression`'s character-level mis-tokenization — garbage in, garbage keywords out, zero real overlap with sibling graphs.
- (a) **Dual-dispatch**: prose-adjacent modalities (math, chemistry, dna, ...) also get their raw content run through text's real entity/topic extraction, in addition to their own specialist pipeline. Most complete fix, but real cost: this session's own text `Analyze` calls (LLM-backed) took up to ~4 minutes each on the free-tier/BitNet fallback path in use tonight — doubling attachment-processing pipeline calls for every math-family file is a real latency tradeoff, not a free win.
- (b) **Supplement `derive_math_keywords` with a lightweight text-style extraction pass over the raw content** (reusing text's own `extract_entities_from_text`/topic-extraction as a library call, or a cheaper heuristic keyword pull) as a second keyword source, blended with or falling back from the `ParseExpression`-derived ones when those look degenerate (e.g. mostly single-character labels). Narrower, cheaper than (a) — only math's own keyword derivation changes, not the dispatch architecture.
- (c) **`LinkToModality`/`TriggerSemanticHook` do not fill this gap as-is** — checked directly (`assets/pipelines/modalities/math/main.rs:415-428`): `LinkToModality` requires the caller to already know both `math_graph_id` and `target_graph_id` (an explicit-link primitive for when the relationship is already known), not a discovery mechanism like `link_related_containers`'s keyword-overlap search. `TriggerSemanticHook` (`ZSEIHookType`) wasn't traced further this pass — worth a look if (a)/(b) are rejected, but not confirmed as a fit.
No implementation chosen — this needs a real go/no-go and which-direction decision, not a unilateral pick.

**A separate, smaller finding while auditing code's node capture**: `CodeAnalysisResult.function_calls` and every `FunctionInfo.calls` come back empty even for source with real calls (verified: `gradient_descent.rs`'s `gradient_descent()` calling `gradient()`, `main()` calling `gradient_descent()` — both real, both invisible in the persisted graph). The schema supports call-graph edges; the tree-sitter query populating them doesn't currently find any. Not investigated further this pass — flagged for whoever picks up code-modality work next.

## 2026-09-22 delta (later same day) — full node/edge/relationship audit, 4 fixes landed, new registry doc is now the source of truth for graph status

**This session's earlier gaps are now substantially corrected — rows above kept for history per this doc's own additive convention, corrections here supersede them where they conflict.** A full 6-fork audit (code/math/text node-edge inventories, cross-cutting relationship structures, S1-S12 graph connectivity, future-modality template survey) plus a follow-on fix wave produced `docs/GRAPH_RELATIONSHIP_REGISTRY.md` — the new permanent, source-verified inventory of every node/edge/relationship type across all 3 modalities plus cross-cutting structures. **That doc is now the authoritative reference for "is this relationship type real" going forward** — this doc stays focused on the doctrine-vs-implementation narrative and delta history, not the exhaustive per-type table.

**Code's call-graph gap (the row directly above, and the status-matrix row) is FIXED, not just found**: `build_graph_nodes_edges` now constructs real `CodeEdgeType::Calls`/`Extends`/`Implements` edges from the already-real extracted data. A fix wave in progress (forks dispatched, not yet build-verified as of this entry) is additionally making `Method` nodes real, which will let `Calls` resolution include method calls too. See `GRAPH_RELATIONSHIP_REGISTRY.md` §1 for the full current table.

**Math's isolation is now substantially, not fully, addressed**: the fabricated `dependencies: vec![i]` (a bug this session found independently of the isolation gap, but related — both were symptoms of `analyze_proof` never deriving real content signal) is replaced with real citation-based `Uses` edges + an honest structural `FollowsStep` edge. `content_keywords` now bridges P4's isolation fix into proof analysis (previously only bare `ParseExpression` calls benefited). **What's still open**: cross-modal linking (§ below) remains purely lexical/keyword-overlap for ALL 3 modalities, not semantic — so math's keyword quality improving doesn't change the fundamentally shallow nature of `SimilarTo` linking system-wide. See `GRAPH_RELATIONSHIP_REGISTRY.md` §2 and §5.

**Text's status matrix row undersold the real gap**: re-verified directly this session — the "different code path" framing (Document/Section/Entity/Topic/Keyword via `CreateGraph` vs. grammar/verb graph via a separate extraction) is real but incomplete. The fuller finding: `TextEdgeType` (~50 variants) has exactly ONE real constructor across the whole file, and it only ever emits `Contains` — every relationship-bearing variant (Contradicts, Elaborates, causal/temporal chains, etc.) is 100% schema-only. The rich vocabulary extracted by the grammar-relationship path (`ChunkGrammarRelationship`, `CoreferenceChain`) is real DATA but was never even schema-typed as a graph edge — it's `edge_type: String`, a raw string, not `TextEdgeType`. A fix wave (forks dispatched, not yet build-verified as of this entry) is building the missing consumer. See `GRAPH_RELATIONSHIP_REGISTRY.md` §3.

**AMT lineage — new capability, not a fix to an existing row**: `RelationType::ForkOf`/`ContinuedBy` now make fork/main lineage genuinely traversable via the same generic `Context.relationships` mechanism gap #1 (above) already fixed for content relationships — confirming that fix's value extends beyond what was originally scoped. See `GRAPH_RELATIONSHIP_REGISTRY.md` §4.

**New structural finding, not previously documented anywhere**: of the system's context sources (S1-S12, `CONTEXT_REGISTRY.md`), only S1/S2/S4/S6/S9 are graph-native; S5/S7/S8 are request-scoped state with no durable container; S10 has a real one-directional bridge (via the consciousness review pass); **S11 (per-call zero-shot metrics, including the `amt_container_id`/`blueprint_id`/`project_id` correlation markers added earlier this session) is a confirmed dead-end — real data, written every call, read by nothing.** A fix (forks dispatched) is building the first real consumer. See `GRAPH_RELATIONSHIP_REGISTRY.md` §5.

**Cross-modal linking is confirmed still purely lexical** (3 independently-maintained copies of `link_related_containers`, one per modality, all keyword/topic-overlap-based, no modality-type filter, no semantic understanding) — unchanged by any fix this session. This is the real ceiling on "how related are two containers actually" system-wide, independent of any single modality's node/edge richness improving.
