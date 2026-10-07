# Context as objects in the graph — the model the ContextBudget service must implement

Status: DESIGN. Nothing here is implemented. This replaces the earlier design's assumption that context is text assembled into prompts. It supersedes the injection-based tool summary.

## 1. The rule

Context is not text carried through the system. Context is a set of objects in the ZSEI graph. A stage holds references to the objects it needs, traverses the graph to find the ones it lacks, and materializes a prompt view only at the moment a model is called. Each point in the flow knows what context it needs, why, and what it actually received.

Corollary: nothing is injected "at all times." A tool list, a jurisdiction rule set, an AMT subtree, or a methodology is fetched by the step that needs it, through a relevance query, and recorded as fetched.

## 2. The objects

Every context source is a ZSEI container (existing store). The kinds that matter:

| Kind | Examples | Source |
|---|---|---|
| Request | the user prompt, its jurisdiction region | stage 0 |
| Plan | blueprint, its steps, step `context_requirements` | stage 6 |
| AMT node | intents, branches, details, cross-refs, methodology assignments | AMT build |
| Chunk | text chunks, sentence and relation outputs | text pipeline (chunk graphs) |
| Methodology | rule text, domain, keywords | methodology store |
| Tool | a registered MCP tool with its capabilities and platform needs | `/mcp/tools` registry (already graph-backed as `McpTool`) |
| Jurisdiction | a rule set, matched rules | stage 0 |
| Simulation | predictions per step | stage 7 |
| Gate and review | consciousness decision, decision-review record | stages 8 and 13 |
| Capture | a call's ContextRecord (below) | every model call |
| Aggregate | an object built from other objects (see section 4) | any stage |

## 3. Relevance is a query, not a list

Each consumer declares its need as a query, not as a pre-built list:

```
ContextNeed {
    anchor: ContainerID,              // where traversal starts (a step, a branch, a project)
    kinds: [Tool, Methodology, ...],  // what kinds are acceptable
    relations: [Contains, RelatedTo, ...],  // which graph edges count as relevance
    depth: u16, budget: TraversalBudget,    // localized: bounded hops, bounded count
    rank: Relevance                    // how to order: relation strength, keyword overlap with the anchor
}
```

Example: the step "send a notification" needs Tool objects whose capabilities relate to notification. The query starts at the step, walks `RelatedTo` edges to capability tags, and returns the matches. The tool list is never sent to a step that does not need tools.

Traversal already exists (`ZSEI::traverse`, `TraversalRequest`). What is missing is the mapping from a consumer's need to a query, and the rule that a stage only materializes what its query returned.

## 4. Aggregation and pass-through

- **Pass-through.** A stage can pass a reference to an object to the next stage without reading its content. Only the call that consumes the content materializes it.
- **Aggregate.** A stage can create an object that contains other objects (`Contains` edges to each input, `AggregatesFrom` to the source). Later stages can extend it. The aggregate is itself context and can be traversed.
- **No silent drop.** An aggregate records every input it included and every input it excluded, with the reason (budget, relevance rank, kind mismatch). Excluded inputs remain in the graph.

## 5. Materialization at the call

A prompt view is built when a model is called, for that model's window:

1. Collect the objects the call's needs returned.
2. Order them by the consumer's priority.
3. Fit to the window of the candidate being called (the walk already knows each candidate's window).
4. Trim at boundaries; record each trim.
5. Send the view; store a ContextRecord linking: the object ids, the view's total tokens, the window, the model, each trim, and each exclusion.

This is the "each call knows what it saw" requirement. It is the reason the materialization happens per call, not once per stage.

## 6. Per-point state

Each stage or sub-stage declares the table below, and the table is kept on the graph so it can be traversed:

| Point | Needs (anchor, kinds, relations) | Produces |
|---|---|---|
| Stage 0 jurisdiction | request, jurisdiction rule sets | jurisdiction result object |
| AMT intent and branch | anchor project AMT, methodology index, related branches | AMT node objects |
| AMT detail and cross-ref | the branch, its chunks, related branches | detail and cross-ref objects |
| Blueprint assignment | AMT subtree, methodologies, tools related to the steps | blueprint object |
| Step execution | the step, its context_requirements, related chunks | step output object |
| Simulation | blueprint, jurisdiction, AMT outline | simulation object |
| Gate and review | AMT, blueprint, jurisdiction, simulation (each by reference) | gate decision object |
| Assistant and I-loop | tasks, reviews, claims, relevant AMT | insight objects |

The table is the operational version of "each point knows what it needs." The current code builds these as strings in prompt templates; this model replaces that.

## 7. What exists today, and what does not

Exists:
- Graph store and traversal with depth and budget (`ZSEI::traverse`).
- Registry of tools as graph-backed `McpTool` entries.
- Relations on containers (`Relation`, `RelationType`) that traversal follows.
- Capture stores for model calls (`zero_shot_calls.jsonl`, `decision_review.jsonl`).

Does not exist:
- A ContextNeed type and a mapping from each stage to its needs.
- ContextRef and aggregate objects. Context moves as strings between stages.
- Per-call ContextRecord with object ids and trims.
- Tool retrieval by relevance. The tool list is precomputed and handed to stages (which is what the user rejected).

## 8. Migration, in order

1. **ContextRecord per call** (no behavior change). Every model call writes the object ids it used, the view size, the window, and the model. This is the capture foundation everything else depends on.
2. **Tool retrieval by need.** Remove the precomputed tool summary from stage prompts. The stage that plans tool use issues a ContextNeed and receives matching Tool objects. Until this exists, stages that planned with tools will lose their tool list. That loss is why this step is paired with the retrieval query, not done alone.
3. **ContextRef for the two largest carriers**: the AMT subtree and the chunk set. Stages pass references; materialization happens at the call.
4. **Aggregate objects** for the blueprint and the gate's review input.
5. **Trim records** on every materialization (replaces the silent caps listed in the context review).
6. **Per-model windows** on every call site (completes the walk's per-candidate sizing, including the `ModelOverrideConfig` gap).

## 9. Open decisions

1. Relevance ranking: relation-strength only, or keyword overlap with the anchor as well?
2. Traversal budget defaults per stage.
3. Whether aggregate objects are persisted always, or only when a stage asks for them.
4. Whether tool retrieval is required before the tool-summary removal lands (recommended: yes, paired).

## 10. What this document does not claim

- That any of section 7's "does not exist" items is built.
- That the traversal budget values are chosen; they are defaults in `types/zsei.rs`.
- That step 2 can run without step 1; step 1 comes first so the removal can be verified against the record.

## 11. Implementation status (2026-10-06, ZCode — CC down)

- **Step 1 (ContextRecord per call) IMPLEMENTED**: src/context_budget.rs — records land as JSONL at `zsei_data/capture/context_records.jsonl` (capture-store precedent, not yet graph objects). Instrumented: every fallback-walk candidate attempt + every S11 loop call. Zero behavior change.
- **Step 2 (tool retrieval by need) IMPLEMENTED at the seam**: `capability_summary_for_need` in orchestrator/stages.rs — blueprint + simulation materialize a per-request view (keyword overlap, every match kept, omitted count stated); the precomputed full-list always-injection is replaced at exactly the two stages that route tools (#46). Full registry remains in state.capability_summary + the graph.
- **Per-model windows gap closed**: the prompt pipeline's ModelOverrideConfig now carries `context_length` (the host already sent it) and merge_override applies it — local fallback candidates finally see their real window.
- **The shared assembler exists**: `assemble()` (priority sections, paragraph-boundary trims, every cut recorded, identity when it fits) is the materialization path the remaining migration (§8 steps 3–6) goes through. NOT yet migrated: the 9 hand-rolled cap sites, ContextNeed/ContextRef, aggregates, trim-marker events to the graph.
- Everything parse-checked, NOTHING built — awaiting the operator's go.
