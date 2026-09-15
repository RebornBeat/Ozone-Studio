> Doctrine: OZONE_STUDIO_SPECIFICATION.md §6 (ZSEI) + §7 (Context Storage).
> Core law: **"ZSEI stores context, not copies."** Everything linked becomes
> a graph node with semantic meaning, relationships, and integrity. The
> graph is the system's memory; traversal — not search-and-concatenate —
> is how context is gathered (§6.7).
>
> This document is the honest status of that doctrine versus the live
> implementation. Updated additively; stale rows get fixed, not deleted.

Last verified: 2026-09-15 (ZCode + Claude Code joint review night)

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
| Text modality graphs | real (sentence/paragraph/section carry-state → containers) | 49+ relationship sites in pipeline | modality containers, cross-process retrieval **BROKEN** (see gaps) | ⚠️ partial |
| Code modality graphs | real (AST → containers, project_id parenting — degrades to root when 0) | relationships array present, per-file | same cross-process gap as text | ⚠️ partial |
| Math modality graphs | real (fixed + live-verified this session, incl. methodology/container relationships) | verified live | works (in-process); cross-process via CLI same open gap | ✅ in-process / ⚠️ cross-process |
| File links (file_link pipeline) | references + analysis JSON under `local/file_analysis` — **not yet ZSEI containers with edges** (ZSEIQuery::LinkFile exists, unwired here) | stored as project references | project file lists | ⚠️ flat |
| URL / package links | same pattern as file_link | same | same | ⚠️ flat |
| Workspaces / Projects | real containers with parenting (live-verified: Workspace 1081 → Project 1082; GetProjects works) | parent/child only — no cross-links yet | task creation, ForStep(project_id) | ✅ core / ⚠️ thin edges |
| Jurisdiction (27 scopes) | real containers under JurisdictionRoot — **flat siblings, no meta-workspace graph, no relationship edges** | none | flat keyword-scan + text dump (the SAST-style failure mode) | ❌ needs graphing (task 56) |
| Methodologies | real containers + real content (3 of 16 with deep content; index of 16 incl. Host Lifecycle) | category links one-directional | methodology search (relevance-fixed) | ✅ / ⚠️ category edges |
| Blueprints | containers registered; content stranded (assets/blueprints missing + old schema) | — | 100%-match reuse path unreachable | ❌ stranded |
| **Coordination graph (NEW, task 42)** | CoordinationEvent containers under /SharedContext root (id 8) — notes/decisions/handoffs/claims, scoped global/ws:/proj: | keywords (kind/agent/file:), claim dedupe; relationship edges v2 | monitor feed today; AMT layer = task 43 | ✅ fresh, edges thin |
| **Context objects (NEW)** | per-step assembled context persisted on the task record (`context_assembled` + `context_sources`) | task-record linkage | GET /task/get steps | ✅ fresh |

## The gaps (root-cause level — all confirmed live)

1. **The TraversalEngine is dead code in the live path** (task 56,
   claude-code). `src/zsei/traversal.rs` implements §6.7 for real —
   Structural / Semantic / Contextual / Hybrid / MLGuided / BruteForce —
   and **zero call sites** exist in any context-assembly path. Everything
   (jurisdiction included) uses flat `SearchContainersByKeywords` + string
   concatenation. This is THE root cause behind weak-model context
   poisoning: no structure tells the model what is relevant versus
   tangential.
2. **`step_contexts` was write-only** (fixed tonight): assembled per-step
   context was never read, persisted, or exposed. NOW: Stage 7 persists
   every step's context object onto the task record
   (`update_step_context` → `context_assembled` + `context_sources`
   provenance, visible in `GET /task/get`). Provenance currently records
   `keyword-scan` — it becomes `traversal:<mode>` when gap 1 closes, which
   makes the context-gathering mechanism itself measurable.
3. **Cross-process graph retrieval** for text/code modality (claude-code
   finding): in-process caches don't survive the CLI boundary — math
   modality's fix is the reference; text/code pending.
4. **Jurisdiction is flat** (claude-code finding, user directive): region
   containers are siblings, not a meta-workspace graph (UN → regional →
   national hierarchy with relationship edges), so jurisdiction context
   can't be traversed or looped like AMT branches.

## The path (active, routed through the host task system)

| Task | Owner | Closes |
| ---- | ----- | ------ |
| 56 | claude-code (lead) + zcode | TraversalEngine wired into context assembly; jurisdiction as meta-workspace graph (first concrete case) |
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
