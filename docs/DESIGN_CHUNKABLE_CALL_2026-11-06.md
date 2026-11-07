# Design — the universal CHUNKABLE CALL (model-agnostic chunking for every model call)

**Status: DESIGN ONLY — nothing here is coded.** Operator ruling (2026-11-06): *"a 17,000-token prompt is never sent to 4,092 … we are model agnostic and context limit doesn't matter, it chunks via all."* Evidence it is not true yet: BitNet received the blueprint-assignment prompt (76 KB / ~19k tokens, `stages.rs:933`) twice and a 163 KB (~40k-token) clock step prompt once (ledger bodies, REVIEW_PROMPT_LATENCY §8). A skip-by-size guard was tried and reverted (refusal by size). This doc is the chunking design to replace it.

## 1. What exists (read from code)
`orchestrator/lane_split.rs` — for AMT **branch-discovery lanes** only. The caller wraps its call in a task-local `LANE_SPEC {members, shared}`; the fallback walk (`mod.rs` ~4003) hands each candidate to `lane_split::run_candidate(window)`: if the FULL rendered prompt fits that candidate's window (`approx_tokens + max_tokens <= window`) it is the unchanged single call; otherwise `place()` halves the members recursively until each part's re-rendered prompt fits, runs each part as its own call, and merges (`branches` arrays concatenated in member order, tokens summed, last model reported). A member too big alone is returned in `lost` — that candidate FAILS and the walk continues (never dropped silently). Properties worth keeping: identity when it fits; per-candidate (a big model gets one call, a small model more); ordered merge; explicit failure instead of loss.
Other chunking already in the system (each site-specific): Stage 2 text chunks (`limit/4` tokens + carry state machines), relation windows (`semantic_relations::windows_for`), decision review (`chunk_task_summary`, any-Decline-wins), network dialogue chunking, AMT branch lanes.

## 2. The gap
Everything else builds ONE flat prompt for the primary's window and the walk hands that same string to every fallback candidate. `metered_execute_resilient` re-derives only `max_tokens` per candidate, never the input. So the doctrine holds only where a site wrote its own chunker.

## 3. Proposed mechanism (generalize `lane_split`, do not replace it)
```
trait Chunkable: Send + Sync {
    fn len(&self) -> usize;                                   // number of members (branches, context sources, items)
    fn render(&self, range: Range<usize>) -> String;          // the FULL prompt for just those members (shared parts repeated)
    fn merge(&self, parts: Vec<PartResult>) -> Result<Value, String>;   // site-specific, ORDER-preserving
    fn strategy(&self) -> Merge;                              // Concat | Accumulate | AnyDeclineWins
}
task_local! { static CHUNK_SPEC: Arc<dyn Chunkable>; }
```
`LaneSpec` becomes one implementation (`Concat`). The walk's existing `if let Some(spec) = current_lane_spec()` generalizes to `current_chunk_spec()`; `place()` recursion is unchanged (halve until `fits(render(range), max_tokens, window)`); a single member that cannot fit alone is reported in `lost` (candidate fails, walk continues) — and for members that are themselves splittable, a second level splits INSIDE the member (see §5).
**Identity rule (the non-drop guarantee, tested):** if the full render fits the candidate's window, the call is byte-for-byte what the site makes today. Chunking changes nothing until a window is exceeded.
**Window source:** the CANDIDATE's own window (`AvailableModel.context_length`, live catalog window for concrete OpenRouter ids). For the `openrouter/free` router the registered 32,000 (REVIEW_WINDOWING W1) — the router case where the real member is unknown stays the open edge (post-call `window_for(served)` is recorded).

## 4. Merge strategies (how "chunk" stays "full context delivered")
| Strategy | Use | Behaviour |
|---|---|---|
| **Concat** | independent members → independent outputs (branch lanes, **blueprint steps**, relation windows) | parts' arrays concatenated in member order; indexes/ids re-based |
| **Accumulate** | the answer depends on ALL context (step execution with a huge context; summarizing) | sequential: chunk k is shown with a short carry (what earlier chunks established — the Stage 2 carry-state pattern), the final call receives the carry + the task. Nothing dropped; more calls for smaller models ("smaller models loop more") |
| **AnyDeclineWins** | review/gate decisions | per-part verdicts; decline wins (existing `decision_review` rule) |

## 5. First two sites
**A. Blueprint assignment** (`stages.rs:933`, "Create a blueprint (execution plan) from this AMT"). Members = AMT branches (each with its detail lines); shared = request, methodology rules, registered tools/providers, pipelines, jurisdiction, STEP TYPES. Merge = Concat: steps in part order, `step_index` re-based by the running count, `depends_on` remapped by the same offset; **cross-part ordering**: the first steps of part k depend on the last step of part k−1 (conservative serial chain) unless the model declared otherwise — *decision for the operator below*; `missing_capabilities` / required providers = union (dedup by name); `reconcile_tool_bindings` already runs AFTER parse on the merged result, so provider steps are inserted once. If one branch alone exceeds the window: split its detail lines (second level) before reporting it lost.
**B. Step prompts** (context_aggregation ForStep → the step's pipeline call; the 163 KB clock prompt). Members = the aggregated context SOURCES (S1–S12 blocks, span-out lines); strategy **Accumulate**: carry notes forward chunk by chunk, final call gets carry + step task. (A step is an action, not a list of independent answers — Concat would be wrong.)
Later sites by the same interface: methodology compliance check, response graph render, I-Loop/assistant digests, meta drafts (all currently `assemble`-trim — W4).

## 6. What it removes / what it keeps
Removes: the ledger failures "prompt is too long (17,625 tokens, max 4,092)" and the cold-load waste behind them; the need for per-site `assemble` trims at the large sites. Keeps: `lane_split` behaviour for branch lanes (now a special case), the walk's order/health/quota logic, honest reporting of unfittable members, ledger/capture of every part (each part is its own recorded call; a `chunked: {parts, window}` field joins them).

## 7. Tests (to write with the code)
Identity harness (fits ⇒ identical single call) for blueprint + step sites · split correctness at windows 4,096 / 8,192 / 32,000 on a recorded 76 KB blueprint prompt · merge renumbering/dependency tests · lost-member reporting · Accumulate carry round-trip · property test: concatenated parts cover every member exactly once.

## 8. Decisions needed from the operator
1. Blueprint cross-part ordering: serial chain between parts (safe, less parallel) vs. independent parts + a final reconcile call?
2. Step prompts: Accumulate (carry notes) — is a model-written carry acceptable as "full context delivered", or must the carry be mechanical (e.g. ids/keys only) so nothing is paraphrased?
3. Granularity of "member" for step context (per source, per span-out line, per node)?
4. Router windows: use the registered 32,000 as the chunk window for `openrouter/free`, or the smallest window observed in the last 7 days?
5. Rollout: blueprint first (highest evidence), then steps?
