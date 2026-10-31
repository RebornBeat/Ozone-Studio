# Review — are the fork edits correct and aligned with the findings? (2026-10-31)

Operator question: "is it correct? review all concerns, is fork 3 aligned with
the new findings, what is missing". Nothing compiled or run; judged by reading
the diffs against the findings.

## Verdict per fork
| fork | scope | aligned? | notes |
|---|---|---|---|
| 1 core scaffolding | Cargo deps, seams->command.rs, JudgmentError vocabulary, guide + parity | yes | follow-up needed: it removed tiktoken-rs but `utils/tokenizer.rs` still imported it (fixed: tokenizer dropped from core, stale import removed) |
| 2 instruction + forms | parser/filler/strategy | yes | `accept_judgment`, `apply_mappings`, `ExternalJudgment` |
| 3 agent + execution | reasoning/agent/navigation/engine/recovery/multi_pass | **yes in design, stopped before finishing** | `ReasoningOutcome::NeedsJudgment`, `EscalationRequest`, no trait/callback, no provider fields. Unfinished: `multi_pass.rs` (finished by me: provider/LlmClient removed; model/chunking/prompt labels left as opaque pass labels, documented), stale `ExtractionRequest` import in engine.rs (removed) |
| 4 extraction/auth/events | extractor rewrite (1500->~640), OAuth as data, wasm JsValue->Value | yes | removed API listed in its report; parity doc must record removed `extract_from_url`/config fields |
| 5 farm unblock | hook auth, graph blocks, registrar, launcher, agent marked unused | yes | `--prune` uses existing `POST /mcp/tools/unregister`; run is an operator/gate step |
| 6 host Rust | `/mcp/call` scope mirror, McpFailureActor | yes | risk list in its report (MutexGuard, broad `refus` marker, terminal noise) |
| 7 proposals | R1-R3 in proposals.rs | yes | route-side half applied by me in grpc/mod.rs + decision_review.rs |

## Concerns found in this review (new)
1. **Four incompatible "judgment request" shapes** (agent `JudgmentRequest`,
   extractor `JudgmentRequest`, engine `ExtractionJudgmentRequest`, recovery
   `EscalationRequest`) plus a second `JudgmentRequired` in engine.rs's local
   error enum. Ozone and the Python wheel need ONE wire shape. Open: add
   `judgment.rs` with `{kind, reason, instruction, context}` and make the four
   convert into it (test AG9).
2. **Never-compiled debt remains** (the removal did not make the crate build):
   `multi_pass` writes `options.model/chunking_strategy/strategy` that
   `ExecutionOptions` does not have; the extractor rewrite assumes signatures
   not compiled; `AuthResult` overloads `error` as a status. All are census
   items; do not call the harvest equivalent until characterisation tests from
   the reference tree pass.
3. **Hybrid-form judgment**: `FillStrategy::Hybrid`/`HostSupplied` rely on
   host-supplied mappings; the Ozone side that supplies them (pipeline 9 +
   S10 review for sensitive forms) has no row until AG8 runs.
4. **No Python library yet**: decision recorded (TOOLS_BY_LANGUAGE §3-5).
   Needs `py/` (PyO3/maturin) and `domorpher-cli`; the single judgment shape
   (1) must exist first so Python sees one contract.
5. **Registry**: local `agent_*` x16 stale rows remain until `--prune`; EU's
   registrar copy and ingest token env need a deploy step; EU has no
   domorpher-agent (intended).
6. **Proposals route**: applied but uncompiled; one-time re-record of open
   proposals expected because review ids now include kind + normalized detail.
7. **Doc drift**: PARITY lists removed extractor/config API only after this
   review; update at the next parity pass.
8. **Process**: fork claims returned `claimed: []` for repo-external paths
   (domorpher), so no claim protected those edits. The shared-context claim
   tool only covers the Ozone tree; domorpher edits are serialized by
   operator discipline.

## Still missing (ordered)
1. Unified judgment contract + `py/` + `domorpher-cli` (blocks T-DOM-GOOGLE D2).
2. Build gate: `cargo check` census on domorpher-core; host `cargo build`
   (scope mirror, actor, proposals, decision_review, route edits).
3. Run order at the gate: AG6a/b -> host build -> registrar --prune ->
   restart farm/browser MCPs with ingest token -> EU deploy (host binary,
   registrar copy, token env) -> T-DOM-GOOGLE D1/D2 -> SC/MF/PR rows.
4. Operator: rotate the OpenRouter key; decide on Browser-Farm repo commit
   b603436 / tools/bridges gitlink.
