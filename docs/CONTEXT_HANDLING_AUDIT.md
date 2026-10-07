# Context Handling Audit — every site, the gaps, and the universal design

2026-10-04. Operator directive: "any model, any context" — the system must
take the FULL context picture into account everywhere: chunking, context
switching, context-fit against model windows, budgets, and metrics tracked on
the graph. Today the logic exists in pieces, hand-rolled per site, with
arbitrary constants and no shared accounting.

## 1. Where context handling EXISTS today

| Site | What it does | Mechanism |
|---|---|---|
| Stage 2 text chunking | Splits input into chunks for graph construction | Text pipeline (modality 100) |
| AMT batch lanes | Token-budget packing (~20k lane budget) + batch_size ceiling | amt.rs parallel helper |
| Host fallback walk | Context-fit pre-order: models that can't hold input+output defer to the back | mod.rs walk_fallback_chain_standalone |
| Text pipeline model rotation | Same context-fit pre-order (chain via OZONE_FALLBACK_CHAIN) | text/main.rs extract_terms_with_fallback |
| Per-candidate output budget | max_tokens re-derived from each candidate's context_length | mod.rs derive_output_budget |
| Prompt pipeline input | `token_budget` field (respects model context_length) | prompt pipeline input |
| AMT lane known-branches | Newest-half guard when >24k chars | amt.rs |
| Methodology summaries | Trimmed to 8k chars | amt.rs |
| Traversal | walk_depth/budget config (max_hops, max_containers) | relevance_policy + traversal budget |

## 2. Where it's MISSING — hand-rolled constants instead of budgets

Every one of these assembles model prompts from bounded slices with
**arbitrary char constants** that ignore the actual model context window:

| Site | Today | Should be |
|---|---|---|
| Consciousness gate (stages.rs) | AMT render capped at 1200 chars | Priority-budgeted section |
| Blueprint prompt (amt.rs blueprint renders) | Branch summaries capped ~120 chars | Priority-budgeted section |
| Cross-reference summaries (amt.rs) | Capped 300 chars | Priority-budgeted section |
| Decision review (decision_review.rs) | 200/500-char cuts per chunk | Budgeted chunks |
| Jurisdiction confirmations | 400-char prompt cut | Budgeted |
| Simulation render (stages.rs) | Simulation predictions joined unbounded | Budgeted |
| Assistant digest findings | **63 findings serialized unbounded** | Budgeted (newest-first) |
| Response assembly (response.rs) | 220/60-char cuts | Budgeted ladder |
| Text pipeline rotation | Now context-fit ✓ (this pass) | — |
| Gate/blueprint tool listings | Not yet injected (queued) | Budgeted injection |

The pattern: every site guesses a char constant instead of deriving from the
target model's window. On a 128k-context model the caps strangle usable
context; on a 4k BitNet the caps are still too generous. Both directions lose
quality.

## 3. The universal design — one Context Budget service

**New canonical module** (host-side `src/orchestrator/context_budget.rs`,
shared structs in `shared/contracts` so pipelines can use it too):

```
ContextBudget {
    model_context: usize,        // from the resolved model (walk gives it)
    reserved_output: usize,      // max_tokens for the response
    safety_margin: usize,        // template scaffolding
}
    .sections(Vec<BudgetedSection>) // (name, priority, text)
    .assemble() -> AssembledPrompt {
        sections_in_priority_order,  // highest priority fills first
        trimmed: Vec<TrimRecord>,    // what got cut, from how much to how much
        total_tokens,                // approx (chars/4), one estimate fn
    }
```

Rules:
- **Priority fills first**: request text and current plan outrank history;
  history outranks summaries. Each section has a declared priority.
- **Trim at boundaries**: sections cut at paragraph/sentence boundaries, not
  mid-token.
- **Metrics on the graph**: every trim emits a marker ("context trimmed:
  methodology-rules 3000→1200 tokens for bitnet-i2_s") — the operator's
  graph-tracked requirement. TrimRecord carries section name, original and
  final sizes, and the model that sized the budget.
- **Never fabricated**: if everything fits, nothing is trimmed and no trim
  events fire.

## 4. Migration (mechanical after the module lands)

Each hand-rolled site swaps its constant for a BudgetedSection with a
priority. Order of migration by pain measured: consciousness gate (AMT render
1200), assistant digest (63 findings unbounded), simulation render,
decision-review chunks, jurisdiction, cross-ref summaries, blueprint renders.
The text/AMT batching sites keep their specialized packing (they're already
token-aware) but re-use the same estimate fn and trim-metric emission.

## 5. Graph tracking

TrimRecord events flow through the orchestration event hub as markers
("[ctx] methodology-rules trimmed 3000→1200 for bitnet-i2_s") — visible in
OrchestrationStatusPanel, persisted in the thinking log next to the response
that was built from the trimmed prompt, and queryable: "what context did this
response actually see?"
