# AMT Expansion Architecture — one gateway, four routes, one candidate store

> Directive: AMT expansion must be CLEAN and UNIFORM — one pathway that
> handles everything (meta work, consciousness, jurisdiction, all
> modalities), not a different path per domain. K-registry pattern: one
> route taxonomy, one candidate store, one review loop.

## The gateway (already clean)

Every orchestration enters AMT building through ONE stage gateway
(`run_amt_stage`), which routes by data readiness:

| Route builder | When |
| ------------- | ---- |
| `build_amt_from_graphs` | chunks carry sentence nodes with grammar relationships (graph-native / OMEX) |
| `build_amt_layer_by_layer` | legacy per-chunk zero-shot loop |

## The expansion routes (why an AMT gets more work)

| Route | Producer | Trigger |
| ----- | -------- | ------- |
| `UnverifiedNode` | amt.rs at build time | persisted project-anchored AMT has a node without source provenance |
| `GraphRipple` | amt_loop.rs::spawn_graph_ripple_sync | graph write scoped to the project (proj:/file:) — the living graph changed |
| `Continuation` | (v2 — designed) | a subsequent prompt expanded the project AMT; prior generation chained via `Continues` root relation |

All routes converge into ONE candidate store (src/orchestrator/
amt_candidates.rs): append (deduped per unhandled AMT), unhandled listing,
load/save. The review loop (amt_loop::review_amt_candidates_once) consumes
every route identically: resolve AMT container → read content file →
find unverified node → model call with methodology guidance + fallback
escalation → deepen → mark handled. Route is provenance, not behavior —
the deepening contract is identical for all.

## The wake model

Event-driven with interval fallback: the graph-ripple sync pokes a Notify
after appending; the review loop selects on Notify OR interval (1800s).
Context alignment latency: seconds after a graph write, not minutes.

## Continuation (v1 LIVE, 2026-09-16 — the main/fork island model)

persist_amt_container now implements the island model:
- **First AMT in a project = MAIN** — keyword `amt-main` on the container.
- **Subsequent generations = FORKS** — islands spawned from the prior
  generation, carrying a root-level `Continues` relation (target = the
  prior AMT's container id) and the keyword `amt-fork-of:<prior>` as
  graph-searchable provenance.
- Route `Continuation` recorded on every fork, so the expansion loop
  knows the lineage context.
- **Merge-back (v2, next)**: substantive fork results graft into the main
  tree (content-match branch grafts); the ripple notifies subscribers of
  the graft. Turns per-request AMT piles into the project's AMT, growing.

## Zero-shot loops + guidance (wired)

The expansion review pass runs with task-62's guidance injected: real
methodology decision-rule text + related-branch content in the deepening
prompt, model fallback escalation cycling on retry, attempt caps. The
graph-ripple sync feeds it event-driven candidates — context alignment
latency: seconds after a graph write.

## Files

- src/orchestrator/amt_candidates.rs — the unified store
- src/orchestrator/amt_loop.rs — review loop + ripple sync + wake
- src/orchestrator/amt.rs — build routes + UnverifiedNode producer
- Tests: amt_loop::tests (T-S1..S5 + lifecycle), amt_candidates paths
