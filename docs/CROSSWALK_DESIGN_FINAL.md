# Cross-walk context transfer — the finalized design
## (2026-10-10, operator-directed; replaces the four-option list in CONTEXT_FLOW_FINDINGS §7)

## The problem (live-confirmed by walks A+B)
Walk A persisted a real 10-branch koi filtration design. Walk B asked about
the same topic. The graph held everything. Walk B's AMT built zero branches
and its model asked for the plan text. **The gap is retrieval-into-prompt:
the enrich phase's keyword search matched (or missed) containers but never
injected their CONTENT into the walk's context.**

## The design: two mechanisms, both using data the graph already holds

### Mechanism 1: Blueprint-content injection
- WHERE: enrich_with_zsei_knowledge, after the existing blueprint search
- WHAT: when a matched blueprint has step content (not just a name),
  inject the steps as text into `knowledge` — the same way methodology
  summaries are injected today
- STORES: nothing new; reads what Stage 6 already persisted
- EFFECT: walk B would have received walk A's plan steps as context

### Mechanism 2: Recency ring
- WHERE: a ring buffer on OrchestrationState (or a small file) carrying
  the last 5 walks': request text, branch titles, and blueprint id
- WHAT: at enrich time, if recent-walk keywords overlap the current
  request's, inject the recent walks' branch titles as "RECENT WORK
  CONTEXT" — the system remembers what it worked on recently without
  requiring the operator to re-state it
- STORES: a JSON file of the last 5 walks' summaries (not containers —
  the containers are already there; this is the retrieval index)

## What this design does NOT do (honest scope)
- Does NOT add embeddings or semantic search (the SemanticSearch wire
  variant stays build-vs-remove for a separate decision)
- Does NOT modify storage (the graph already has the data)
- Does NOT require model calls (pure retrieval + assembly)
- Does NOT change the intent/lane/cross-ref dispatch mechanics

## Build order
1. Mechanism 1 first (smallest, fixes the live-confirmed gap)
2. Mechanism 2 second (cross-walk memory)
3. Both verified by a walk B3 retry (the same koi review prompt)
