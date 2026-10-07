# Context Budget Service — inventory, design, migration

Status: DESIGN for review. Nothing here is implemented. Every inventory row was read in the code on 2026-10-06. Operator directive: "any model, any context" — the system takes the full context picture into account at every step, and nothing is dropped.

## 1. Inventory: every chunker, estimator and budget that exists today

### 1a. Token estimators (six, all different)

| # | Location | Formula | Used for |
|---|---|---|---|
| E1 | assets/pipelines/modalities/text/main.rs:2463 `estimate_tokens` | `(len + 3) / 4` | Text chunk accounting |
| E2 | assets/pipelines/modalities/text/main.rs:4088 (rotation pre-order) | `len / 4 + 1` | Text model rotation fit |
| E3 | src/orchestrator/amt.rs:2253 `approx_tokens` | `len / 4 + 1` | AMT lane batch packing |
| E4 | src/orchestrator/mod.rs:3091 walk pre-order | `(prompt + system) / 4 + 1` | Host fallback walk fit |
| E5 | src/orchestrator/mod.rs:3786 `estimate_tokens` | `(len + 3) / 4` | Orchestrator accounting (callers not traced) |
| E6 | assets/pipelines/general/prompt/main.rs:217 `chars_per_token = 4` | constant | Prompt pipeline budget |
| E7 | assets/pipelines/general/text_analysis/main.rs:392-418 | `len / 4` | Text analysis paragraph and sentence packing |

Consequence: the same prompt can be judged to fit by one site and not by another. There is no single answer to "how big is this text".

### 1b. Chunkers and packers (three independent chunkers)

| # | Location | Unit | Budget source | Model-aware? |
|---|---|---|---|---|
| C1 | text/main.rs:2499 `chunk_text` | chars (from `max_chunk_tokens * 4`) | orchestrate path: `model_context_limit / 4` (mod.rs:2000), dynamic per request. The fixed `2000` (text/main.rs:1405) is only the default for callers that omit the field (contract tests, other analyzers). | **Yes** on the orchestrate path. The fixed default is a caller fallback, not the main path. |
| C2 | src/orchestrator/amt.rs:2256 lane packing | approx tokens | was a fixed `20_000` (added in commit f752f14); now `3/4 × model_context_limit` | **Yes** after this pass. Originally, before f752f14, lanes were packed by member count only. |
| C3 | general/text_analysis/main.rs:392-418 | paragraphs, then sentences, by `len/4` | fixed | **No.** Pipeline 20, registered in src/pipeline/registry.rs:50. Call sites not traced. |

Also present: `reconstruct_context_at_token_limit` (text/main.rs:7527), which trims to a token target and cuts at a sentence boundary. **It has no callers.** It is the closest existing piece of the budget service and must be kept, not deleted.

### 1c. Budgets and caps that are constants (no model awareness)

| Site | Location | Cap |
|---|---|---|
| Consciousness gate AMT render | stages.rs ~1400 | 1200 chars |
| Blueprint branch summaries | amt.rs ~777 | 120 chars |
| Decision review chunks | decision_review.rs ~240, 247, 332 | 200 / 500 chars |
| Known-branches guard | amt.rs ~2282 | was 24 000 chars; now `window_chars / 4` (window_chars = 4 × model_context_limit) |
| Methodology block | amt.rs ~2291 | was 8 000 chars; now `window_chars / 12` |
| Jurisdiction prompt | jurisdiction.rs:511 | 400 bytes (char-safe after this review) |
| Response ladder | response.rs:147, 271 | 220 / 60 chars |
| Simulation predictions | stages.rs ~1420-1432 | **uncapped** |
| Assistant digest findings | consciousness/assistant.rs ~338-353 | **uncapped count** |
| Registered-tools summary (now delivered) | stages.rs:146-150, built mod.rs ~1618 | **uncapped**, one line per tool, 121 today |
| Pipeline token_budget default | grpc/mod.rs ~1893 | flat 100 000 |
| Simulation step budget | stages.rs ~1917 | `model_context_limit / 4` — the one model-aware budget outside the walk |

### 1d. Where context switching per model happens today

| Step | Per-model behavior | Location |
|---|---|---|
| Host fallback walk | Candidates that cannot hold input + output are deferred to the back, never dropped | mod.rs ~3061-3074 |
| Host fallback walk | `max_tokens` re-derived from each candidate's own `context_length` | mod.rs ~2754-2761 |
| Host fallback walk | Health reorder: failing models move to the back | mod.rs walk head |
| Text rotation | Same context-fit pre-order (E2) | text/main.rs ~4081-4099 |
| Processing path choice | `model_context_limit` selects the AMT path | mod.rs ~1968 |

### 1e. Where context switching per model is MISSING

- **Chunk sizes** (C1, C3) are fixed. When the walk switches to a model with a larger or smaller window, the chunks do not change.
- **Lane batches** (C2) are packed once for 20k. A lane that walks to a smaller model fails the fit check and is deferred, but its batch is never re-packed for that model.
- **Step prompts and AMT render blocks** use constants (section 1c), not the window of the model that will read them.
- **Per-step model overrides** (blueprint `model_override`, stages.rs ~511-562) change the model for a step, but the step's context is not re-fitted to it.
- **Five estimators disagree**, so a fit decision in one place does not carry to another.

## 2. Design

### 2.1 Placement

- Host module `src/orchestrator/context_budget.rs`.
- Pipeline copy `assets/pipelines/shared/context_budget.rs`, included with `#[path]` exactly like `semantic_relations.rs`. Pure std and serde_json, no host dependencies. Host and pipelines must share one implementation, so the file is the single source and the host re-exports it.

### 2.2 Types

```rust
pub struct ModelWindow { pub identifier: String, pub context_tokens: usize, pub max_output_tokens: usize }

pub struct TokenEstimate { pub tokens: usize, pub method: &'static str }   // method names the estimator used
pub fn estimate(text: &str, profile: TextProfile) -> TokenEstimate          // ONE function for all sites

pub struct Section { pub name: &'static str, pub priority: u8, pub text: String, pub required: bool }

pub struct TrimRecord { pub section: &'static str, pub from_tokens: usize, pub to_tokens: usize, pub model: String }

pub struct Assembled { pub text: String, pub included: Vec<&'static str>, pub trims: Vec<TrimRecord>, pub total_tokens: usize, pub budget_tokens: usize }

pub fn assemble(window: &ModelWindow, scaffold_tokens: usize, sections: Vec<Section>) -> Assembled
```

Budget = `context_tokens − max_output_tokens − scaffold_tokens − safety_margin`. Required sections are never trimmed; if they alone exceed the budget, the call returns `Err(Overflow)` and the walk defers the candidate. It never truncates a required section silently.

### 2.3 Rules

1. **Priority fills first.** Sections are included in priority order. Lower-priority sections are trimmed first.
2. **Trim at boundaries.** Cuts happen at paragraph, then line, then sentence, then space, using the existing rule in `chunk_text` (C1). Never mid-word, never mid-character.
3. **Identity when it fits.** If everything fits, `text` is byte-for-byte what the caller would have built today, and `trims` is empty. This is the test that guarantees nothing changes until a budget is actually exceeded.
4. **Nothing is removed from the graph.** A trim changes only the prompt view. The full content stays in its container; the trim record says which part the model did not see.
5. **Deferral, not skipping.** Overflow moves a candidate to the back of the walk, as the host walk already does.

### 2.4 Chunking (replaces C1, C3 and `reconstruct_context_at_token_limit`)

- `chunk_for_window(text, window)` returns chunks sized to the window's input budget, not a fixed 2000 tokens.
- The existing C1 boundary logic is moved, not rewritten: the 400-byte break window and the paragraph, line, space order stay as they are.
- Changing chunk size changes the persisted chunk graph. **This is a decision**: keep fixed chunks as the default, and let window-sized chunks be opt-in per call site.

### 2.5 Lane packing (replaces C2)

- A batch is packed to the budget of the **smallest candidate the batch may land on**, which is known from the walk's order. Larger models still receive the batch.
- If a batch no longer fits a candidate reached later in the walk, that candidate gets a repacked batch from the same members. No member is lost: each member lands in exactly one batch per attempt, as today.

### 2.6 Metrics on the graph

Every `assemble` call emits one `ContextRecord`:

```
{ call_site, step_id, model, window_tokens, budget_tokens, total_tokens, method,
  included: [...], trims: [{section, from, to}], deferred_candidates: [...] }
```

- Carried through the orchestration event hub as a `[ctx]` marker (one per call, not one per trim, to keep volume bounded).
- Persisted as a container linked to the step, so "what context did this response see?" is a traversal.
- The zero-shot capture rows (S11, `zero_shot_calls.jsonl`) gain a `context` field carrying the record id. The record and the row point at each other.
- Observed token counts from model responses (`usage`) are stored next to the estimate, so the estimator's error is measured per model. Calibrating the ratio per model is then a data question, not a guess.

### 2.7 Full context flow (sources → sections → model → capture)

| Source | Section | Priority (proposal) | Today |
|---|---|---|---|
| Request text and current plan | `request` | 100, required | Full, uncapped |
| Blueprint steps | `plan` | 90 | Full |
| Jurisdiction summary | `jurisdiction` | 85 | Full |
| AMT render (branch tree) | `amt` | 80 | 1200 chars at the gate; full elsewhere |
| Related chunks (`related_chunk_indices`) | `related` | 70 | Pulled in full |
| Methodology rules | `methods` | 60 | 8000 chars |
| Simulation predictions | `simulation` | 50 | Uncapped |
| Registered tools | `tools` | 40 | Uncapped (new, delivered in this pass) |
| Reflections, digest, history | `history` | 20 | Uncapped digest |

The priorities are a proposal. They are the decision the operator must make per section.

### 2.8 AMT expansion and branches

- Branch nodes already carry chunk references. Expansion (amt_loop reexpansion, branch generation) assembles its prompt through `assemble`, with the target branch at the top priority and related branches below it.
- Methodology-domain and synthesis calls get the existing methodology list as a section (see the ZCode context audit, sites 7-8), at a priority below the request.
- Cross-reference summaries have no cap in the code today (ZCode's 300-char claim is unsupported). Their section gets an explicit budget.

## 3. Migration (behavior-preserving, in order)

1. **Estimator and records only.** One `estimate` function; `ContextRecord` emitted; no trimming. Test: outputs unchanged, records present.
2. **Identity harness.** For each site, a test that the assembled text equals today's text when the window is large. This is the non-drop guarantee, checked, not promised.
3. **Consciousness gate** (1200-char AMT render, simulation, tools). Test: a 4k window trims and records; a 128k window is identical to today.
4. **Digest, simulation, decision review, jurisdiction, cross-reference, blueprint renders.**
5. **Lane packing per candidate** (C2) and **text chunking per window** (C1, C3), last, because they change persisted content and need the decision in section 2.4.
6. **Tool summary cap** (section 1c) before the registered-tools section ships.

## 4. Decisions needed from the operator

1. Chunk size: fixed 2000 tokens (today) or window-sized (opt-in per call site)?
2. Priorities per section (section 2.7).
3. Lane budget: per smallest candidate, or fixed with repack on fallback?
4. Tool summary cap: top N by relevance to the request, or a fixed count?
5. Grpc pipeline `token_budget` default: 100 000 flat, or derived from the request's model?

## 5. Risks

- **Estimator error.** chars/4 is wrong for code, math and CJK. Mitigation: record the method and observed usage; calibrate per model before enforcing tight budgets.
- **Repack cost.** Repacking lanes per candidate costs CPU per fallback. Mitigation: pack once per distinct window in the walk, not per attempt.
- **Persisted content changes** when chunk sizes change. Mitigation: opt-in per call site, identity harness on fixed chunks first.
- **Event volume.** One record per call is bounded; per-trim markers are not emitted.

## 6. What is not in this document

- The implementation. Nothing here is built.
- Calibration numbers. None are measured yet.
- The per-section priorities. They are proposals for the operator.

## 7. History and correction (2026-10-06)

- The text chunker's `max_chunk_tokens` field has existed since the text pipeline was first written (commit b4c106d, 2026-03-24). Its default of 2000 was always a fallback. The orchestrate path has always passed a window-derived value, now `model_context_limit / 4`.
- The AMT lane token budget (20 000) came from commit f752f14. Before it, lane batches were sized by member count only (`policy.batch_size`). The 20 000 cap was a fixed constant that stood in for a window-derived budget. It is now window-derived, so it stops limiting large-window models.
- The known-branches (24 000) and methodology (8 000) caps arrived in the same commit. They are now fractions of the window.
- Still open: a lane whose composed prompt exceeds a smaller fallback candidate's window is deferred by the walk, not split. Splitting a lane at call time needs the lane prompt built from a member list, which is a refactor of lane construction. Until that is done, the walk's deferral is the only per-candidate protection.
