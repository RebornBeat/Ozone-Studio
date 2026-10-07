# Intent path review — does the user's intent survive to the blueprint?

Status: read-only review by Claude Code, 2026-10-06. Nothing was built, tested, edited in source, or sent to an LLM. Every line reference below was read in the tree.

Purpose: check ZCode's explanation of the quality degradation ("displacement chain, not a drop", handoff muww... / muwvpht91c53) against the code path from request to blueprint.

## 1. Verdict

ZCode's mechanism is partly right and incomplete.

- **Right:** the blueprint can end up built on branches that are not the user's intent. Nothing is dropped in the lane or retry layers.
- **Incomplete:** the blueprint never sees the user's request text. In multi-intent requests it does not see the branch text either. Two fallbacks can replace the real intent with generic text, and two more can silently discard or reattribute branches. None of these is a record in the graph.
- **Not verified:** ZCode's specific claim that weak-model branch generation produced 5 branches that parroted methodology titles. I did not read that run's data. The mechanism below would allow it; the run itself is unchecked.

## 2. The path, with the places where the intent can be lost

1. **Intent extraction** (`src/orchestrator/amt.rs` ~1169–1260). An LLM call returns `{"new_intents": [...]}`. Parse failure is silent: `unwrap_or_else(|_| json!({"new_intents": []}))` at amt.rs:1219. Zero intents is then accepted without a record.
2. **Intent fallback, graph path** (amt.rs:905–915). If `intent_captures` is empty, a default intent `"Process user request"` is pushed. The user's text is not used.
3. **Intent fallback, second path** (amt.rs:1290–1299). The same generic default.
4. **Branch extraction from LLM output** (amt.rs ~1556–1605, and a second path at ~2494–2551). Each branch's `parent_intent` is resolved against intents by two-way substring match. If nothing matches, it falls back to `intent_captures.first()` (amt.rs ~1581–1593 and ~2508–2521). An empty `parent_intent` string matches every intent via `contains("")`, so it always resolves to the first intent. The match is silent.
5. **Branch dedupe** (amt.rs ~1563–1571 and ~2526–2529). `already_exists` uses two-way substring matching against existing branch text. A short generic branch (for example a methodology title) is treated as a duplicate of any longer branch that contains it, and is dropped without a record. Equally, a branch that contains a short existing branch is dropped.
6. **Branch quality pruning** (call at amt.rs:1044–1047; `score_branch_quality` at ~2769; `apply_branch_quality` at ~2820–2840). Branches with `total_score < 0.15` and no methodology ids are removed with `retain`, along with their detail captures. No record is written. The tracing line is `debug!`, not a graph event.
7. **AMT assembly** (`assemble_amt_from_captures`, amt.rs ~1761–1860).
   - Multi-intent: intents become children of the root. Branches attach to an intent only when `bc.parent_intent == intent.intent` (exact string equality, amt.rs:1820). Any branch whose parent string does not exactly equal an intent is never attached and never appears in the AMT.
   - Single-intent: branches attach directly to the root, with the same exact-equality filter (amt.rs:1834).
8. **Blueprint input** (`stage_3_blueprint_assignment`, `src/orchestrator/stages.rs` ~344–620). Two gaps:
   - The prompt does not contain the user's request text. Only `amt.content` (the root intent text) and the root's children are used.
   - The children are listed as `- {c.content}: {c.children.len()} children, chunk refs: [...]` (stages.rs ~602–612). In multi-intent mode the children are intents, so **branch text is never shown to the blueprint**; it sees intent names and child counts. The prompt still says "every branch listed above must be addressed", which the model cannot do from counts alone.

## 3. What this means for "displacement"

- **Intent displaced by a generic root.** If extraction returns nothing (step 1 or 2), the root is `"Process user request"` and the blueprint has no other source for the request. This is the strongest candidate for "the intent was displaced." It is silent.
- **Branches displaced by reattribution.** A branch whose `parent_intent` does not match falls to the first intent (step 4). The blueprint then sees one intent with more children. Nothing is lost, but the attribution is wrong and the audit trail says nothing.
- **Branches dropped by dedupe or pruning.** Steps 5 and 6 remove branches with no record. A user reading the graph cannot tell whether a branch was never generated or was removed.
- **Blueprint blind to branch text in multi-intent mode.** Whatever branches survive, the blueprint cannot see their text. Steps then get drafted from the root and the child counts. ZCode's "blueprint took exactly those 5 branches" would need the blueprint to have seen their text, which the code does not support in multi-intent mode. In single-intent mode the branch text is visible.

Conclusion: the observed symptom (methodology-flavoured steps, generic output) matches the blueprint blind spot plus the generic root. "Nothing was dropped" is not accurate. Branches can be removed without a record (steps 5–6) and the intent can be replaced without a record (steps 1–3).

## 4. Proposed fixes (not implemented; for approval)

1. **Never replace the request with a generic intent.** If extraction yields no intents, use the request text (`state.cleaned_prompt` or `state.request.prompt`, char-safe trimmed) as the intent, and record a fallback event. Remove `"Process user request"`.
2. **Carry the request and every branch into the blueprint.** Add the request text and the root intent to the blueprint prompt. In multi-intent mode, list each intent with its branch texts (grandchildren), not child counts.
3. **Remove the first-intent fallback in parent resolution.** An unmatched or empty `parent_intent` should be recorded as unattributed (with the raw string), not attached to the first intent. Ask the model for an exact intent index or id in the lane output, and fall back to fuzzy matching only as a recorded step.
4. **Replace substring dedupe with normalised equality** (lowercase, trimmed, collapsed whitespace). Record each merge or duplicate as an event, with the branch it merged into.
5. **Record every prune.** `apply_branch_quality` should write each removed branch and its score to the capture store (or a trim-style record) instead of `retain` alone.
6. **ZCode's intent-alignment check** (branch validation requires a branch to relate to a stated intent, and rejects branch text that copies a methodology title) is sound in direction, but it needs fixes 2 and 5 first; otherwise the rejections are invisible and the blueprint still cannot see the branches.

## 5. Checks not done

- ZCode's run data (the 5 parroted branches, the 39-minute run) — not read.
- Whether `parent_intent` mismatches actually occur in recent runs — would need the capture data; not checked.
- The `already_exists` substring case is a code reading; its frequency is unknown.
- No build, no test, no LLM call, no production store write.

## 6. Related open items

- Loop call sites recording window 0 (`capture_loop_model_call`): ZCode's note says the window is "unknown at this seam" and the fix is to resolve from `model_used`. Separate from this review.
- BitNet global `[models]` block context_length 128000 versus the per-model 4096 entry (config.toml ~86–96). Needs an operator decision before any loop uses the global value.
