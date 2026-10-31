# Review — the domorpher library boundary (2026-10-30)

Scope: everything ZCode has done on the DOMorpher conversion (Ozone commits
`a72fc19..cf5a07c`, domorpher repo `8b03f6d`), checked against the operator
rule: **domorpher is a crate/library — a DOM handler with a contract. It has
no LLM, no model client, no server, no storage. Intelligence lives in
Ozone-Studio and reaches the library as DATA.** Review only: no source edits,
no builds, no tests were run for this document.

## 1. What is already right (keep, do not redo)

- `DecisionSource` is gone from `core/src/seams.rs`. Only `PageDriver` and
  `StateStore` remain. The module comment states "the library receives
  decisions as DATA".
- `core/src/lib.rs` already leaves out `llm/ api/ enterprise/ monitoring/
  storage/ pipeline/ parallel/`, the browser drivers, and `extension/`.
  `chunking/` was retired by the taxonomy test (it duplicates text pipeline
  100). `agent/` is commented out until its split.
- `core/src/utils/mod.rs` declares only tokenizer/encoding/config. `http.rs`
  and `logging.rs` are left out.
- `tools/agent-mcp` is now `tools/domorpher-agent`, with 16 verbs, locked by
  default, honest "harvest pending" refusals, farm backend by config. The
  intelligence stays in Ozone: the MCP is the enforcement edge, not a brain.
  Keep it general, as the operator asked.
- Ownership matrix §2.0e (14 roles, one owner each) is correct in substance.

## 2. What was missed or is still wrong

### M1. `core/Cargo.toml` still contradicts AG6 (P0)
The header says "nothing that opens a socket … or talks to a model", but the
dependency list carries:
| dep | why it is wrong for a pure library |
|---|---|
| `reqwest` (json, rustls-tls) | an HTTP client. Only `utils/http.rs` (OUT), `extraction/extractor.rs` (split-pending), and `browser/auth.rs` use it. |
| `jsonwebtoken` | auth token handling belongs to Ozone's session layer |
| `tiktoken-rs` | model-tokenizer coupling. A library should count with a trait or a plain estimate, not a vendor tokenizer. |
| `wasm-bindgen` | no wasm target is in scope for the core |
| `tokio` `rt-multi-thread` | a library should not start a runtime; `sync,time` features suffice |

These were added to silence resolution errors, which hid the real fix: remove
the call site. AG6 ("the resolved dependency tree has no http/llm/storage
crates") currently **fails**. Add a `cargo tree` check as the AG6 test itself.

### M2. The IN-list is still entangled with the model in 7 files (P0)
Declared IN-list modules that still contain model-client code, by weight
(count of `crate::llm|LlmClient|LlmProvider|reqwest|llm_` references):

| file | lines | refs | the model call site | split (decisions as data) |
|---|---|---|---|---|
| `extraction/extractor.rs` | 1500 | 26 | LLM zero-shot extraction, HTTP fetch | keep CSS/XPath/regex/semantic extraction; zero-shot becomes `ExtractionMethod::LlmZeroShot` **as a plan label** executed by Ozone (S11 zero_shot capture) and the result is fed back as `Vec<ExtractedItem>` |
| `execution/recovery.rs` | 946 | 26 | LLM-suggested recovery | keep detection + retry/backoff state machine; the model's "alternative action" arrives as an `ActionDecision` argument |
| `instruction/parser.rs` | 1120 | 14 | `parse_with_llm`, prompt builders, JSON-from-LLM extraction | keep rule-based parse; delete `llm_client`, `with_llm_client`, prompt creation, `parse_llm_response`. Ozone (pipeline 9) produces a `ParsedInstruction` and passes it in. Keep the `extract_json` helper only if the rule path uses it. |
| `agent/reasoning.rs` | 2403 | 9 | direct `LlmClient` calls in strategies | pure strategies `(perception, instruction, config) → ActionDecision` stay; the LLM strategy is removed from the crate; Ozone supplies a decision |
| `execution/engine.rs` | 905 | 9 | holds an LLM handle | drop the field; the engine takes decisions |
| `forms/filler.rs` | 847 | 8 | `with_llm_provider`, AI-guided field mapping | keep deterministic mapping; AI mapping becomes an input `Vec<FieldMapping>` |
| `execution/multi_pass.rs` | 1487 | 6 | escalation picks a stronger model | escalation is a **signal out** (`NeedsStrongerJudgment{reason}`); Ozone picks the model and re-enters |
| `browser/auth.rs` | 884 | 2 | reqwest | keep the state transforms; drop network calls (login is a farm action) |

Rule for every split: **delete the model call, keep its input and output
types.** The output type (ParsedInstruction, ActionDecision, FieldMapping,
ExtractedItem) is the contract. Do not replace the call with a trait or a
callback: that re-creates `DecisionSource` under another name. A plain
function argument or an enum "needs judgment" return value keeps the library
inert.

### M3. "LLM" as a vocabulary leak
`ExtractionMethod::LlmZeroShot`, `DOMorpherError::LLMError`,
`LLMMappingFailed`, the "AI-guided" prose, and prompt templates in
`src/llm/prompts` (reference only). Rename to neutral contract words
(`ExternalJudgment`, `JudgmentRequired`) so the library API does not imply a
model lives inside. The parity manifest must map each one to the Ozone pipeline
that now answers it.

### M4. Docs contradict the corrected seam
- `DOMORPHER_CONVERSION_GUIDE.md` §2.0a (lines ~97-146) still describes the
  old design: `decision_source` parameter on reasoning strategies, the
  `DecisionSource` trait seam, "chunking/ IN", and a "three seams" list.
  §2.0e and the seams.rs comment say the opposite. Correct §2.0a in place; do
  not delete the history, mark it superseded.
- Line 201 ("Seam impls: DecisionSource over pipeline 9") likewise.
- `DOMORPHER_CAPABILITY_PARITY.md` line 12 lists `chunking/` as IN core
  (retired), and the "~66k IN core" total includes it. Recompute honestly.
- `MASTER_TEST_GUIDE.md` §7c: AG6 needs the concrete check (M1) and a row that
  the crate's public API contains no `Llm*` type (grep gate).

### M5. The "unused" MCP and the real route
`domorpher-agent` is described as "currently UNUSED (the farm executes)".
Then it is not yet the enforcement edge for anything. Either (a) route every
farm-executed library action through it (`/mcp/call` -> domorpher-agent ->
farm), which keeps one audit point, or (b) state explicitly that the farm
tools (`farm_*`) are the enforcement edge and domorpher-agent is only the
verb vocabulary. Pick one in the guide. Today the guide says both.

### M6. Ozone side: where each removed LLM call is answered
For nothing to be dropped, each removed call needs a named Ozone answerer and
a test. None of these rows exists yet in MASTER_TEST_GUIDE:

| removed from library | answered by | evidence to add |
|---|---|---|
| instruction understanding | pipeline 9 decision, or the zero-shot path (S11 capture) | row: NL instruction -> ParsedInstruction passed into library; S11 row exists |
| extraction zero-shot | zero_shot_calls (S11), image/text modality analysis | row: extractor returns same items given Ozone-supplied result |
| recovery alternative | acting chain: TaskFailed actor -> candidate -> amt_loop | row: failed farm action produces a CoordinationFinding (this actor does not exist for browser/farm/android failures, see REVIEW_PROPOSALS R-list) |
| model escalation | `OZONE_*` model routing in pipeline 9 | row: `NeedsStrongerJudgment` -> re-enter with new decision |
| form field mapping | pipeline 9 + capture S10 review | row: proposed mapping goes through `Proceed` for sensitive forms |

### M7. Process observations on the campaign
1. The core census oscillated 63 -> 704 -> 621 -> 301 -> 145: fixes surface the
   layer beneath. Cheaper path, which the operator already named: apply the
   taxonomy test and the model-call removal (M2) **before** more error-grinding.
   Every LLM-importing file is the same cascade.
2. Several types were recreated from use sites (error.rs 62 variants,
   internal.rs, TokenCounter, html_text). Those are plausible but unverified
   against original behaviour; they need characterisation tests from the
   reference tree before the harvest is called equivalent.
3. `reqwest`/`tiktoken` were added during the grind; each dep addition should
   pass the AG6 question in the Cargo.toml comment, and did not.
4. Cargo.toml has two identical "Own workspace root" comments and an empty
   `[workspace]` at the end; tidy when M1 lands.

## 3. Removal plan (ordered; edit-only until the build gate)

1. Cargo.toml: drop `reqwest`, `jsonwebtoken`, `tiktoken-rs`, `wasm-bindgen`;
   trim tokio features. Expect the census to rise at the call sites in M2.
   That is correct: those lines are the work.
2. For each M2 file: delete model call sites, keep types, expose "judgment
   required" and "decision supplied" as data. Order: parser -> filler ->
   reasoning -> recovery -> multi_pass -> engine -> extractor.
3. Re-include `agent/` (pure part), `forms`, `instruction` in the core only
   after their split.
4. Rename the `Llm*` vocabulary (M3).
5. Correct the docs (M4), add the AG6 `cargo tree` check and the "no `Llm`
   in the public API" grep gate to MASTER_TEST_GUIDE §7c.
6. Add the M6 rows (Ozone-side answerer per removed call).

## 4. Insights

- The reference tree's "AI" features are model calls sandwiched between pure
  logic. Cutting them removes the cascade, not capability. The capability
  moves up to pipeline 9 and the acting chain, which already meter, gate, and
  review it.
- A library with decisions-as-data is also trivially testable offline, which
  fits the operator rule "no mocks": feed real recorded decisions (S10/S11
  captures) into the library and assert the same DOM actions result.
- Browser-Farm hosts the crate, so the Command Center channel is where
  decisions arrive. The decision payload should be the same S10 review-row
  shape the proposals chain uses, so "Proceed" has one meaning everywhere.

## 5. Open for the operator

- Approve editing the domorpher repo (ZCode is mid-grind there; I left it
  untouched). My recommendation: ZCode does steps 1-2, I review each split.
- Whether `domorpher-agent` is the enforcement edge (M5a) or a vocabulary
  only (M5b).
