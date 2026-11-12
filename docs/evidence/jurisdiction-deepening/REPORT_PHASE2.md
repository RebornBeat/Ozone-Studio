# Jurisdiction deepening, phase 2 — report (2026-11-12)

Commit a4a22a7 (code, parse-only: rustfmt / node --check / ts.transpileModule; nothing compiled or run; EU untouched).
Engineering input, not legal advice. Checklist lines: CHECKLIST_LINES_PHASE2.md (L190–L203).
Report text returned by the implementing fork (harness refuses fork-written reports) and written by the parent.

## Gaps closed (from REPORT.md §9)
| Gap | Change |
|---|---|
| G2 `/pipeline/execute` | Runs the same Block/Warn/Log/RequireConfirmation review as `/mcp/call` via shared `jurisdiction_review_for_call` (`grpc/mod.rs:6446`); every outcome is an append-only decision row. |
| G3 tool routing | `OZONE_TOOL_ROUTING` on by default (`tool_routing.rs:69`); call and step gates installed synchronously at boot from the compiled Global rule set (no uninstalled window). |
| G4 agent steps | `policy::check_outbound_to_agent` (`policy.rs:801`) matches the agent endpoint against policy `providers`; unlisted agent = unverified region; call holds with 2 s–60 s backoff until the operator clears it. |
| G5 publish kinds | `p2p.publish_kinds` enforced on announce, list and fetch-serve. New offerable kind `jurisdiction_policy` (door vocabulary 4 → 5), published only if listed and `share_policy_as_offer` is on; receiver only stages it; adopt stays the explicit `/jurisdiction/policy/adopt`. Operator offer: `POST /jurisdiction/policy/share` + UI. |
| G6 search bridge | Query carrying email/phone/IBAN/card shape is held, not offered to any answering session. `search_release` is operator-only: `OZONE_OPERATOR_RELEASE=1` + `confirm:"RELEASE"`. |
| G7 bridges | `p2p-bridge-mcp` carries no user content. `tools/bridges` (git submodule) `llm-proxy.ts` / `providers.ts` call Anthropic, OpenAI, Google, z.ai, MiniMax directly, bypassing the host gate. NOT edited. |
| G9 snapshot | Gate snapshot rebuilds on policy save/adopt, content sync, region population, plus the 600 s timer. |
| G11 held calls | `policy::held_backoff` paces retries (250 ms doubling to 30 s), ends early on operator decision, never turns a hold into an allow. |
| G12 region | `GET /jurisdiction/policy` returns `instance_region` + `region_known`; UI banner when unknown. |

## Confidence removal finished
`GateResult.confidence` and `DecisionReviewRow.confidence` deleted; constant `"confidence": 1.0` on the stage-13 edge write removed. `Relation.confidence` is `serde(default, skip_serializing)`: legacy edges read, absent reads as 0.0, never written back (field kept because ~20 literals/fixtures use it). Any consumer expecting `relationships[].confidence` in serialized JSON now sees it absent.

## Durable-write decisions applied
- Snapshot option A: a container whose load fails is snapshotted from raw bytes (`raw_local_bytes`); mutation (including Delete) proceeds with a real restore point. A failing rollback directory still pauses mutations loudly (`durable_write_stalled`).
- Stage 13 delivers first: a `wo_link` child item opens under the walk's task before delivery; the completion gate waits on it; delivery runs, then the link pass runs in the background with retries and ticks the item. Both stage paths use `deliver_then_link`.

## Open for operator decision
1. `tools/bridges` direct provider calls: route through the host or patch that repo.
2. Background link pass is lost if the process dies mid-pass; `wo_link` stays open and the walk stays parked visibly; operator cancel releases it. Boot-time resume needs the pass inputs persisted.
3. `held_backoff` makes held calls wait up to 30 s per retry; tests expecting an instant error must tolerate it.
4. G1 (in-region vote model), G8 (retention job), G10 (store-held erasure) unchanged.

## Check first at cargo check
Deref coercion in `jurisdiction_review_for_call(&runtime, …)`; `check_outbound_inner` split and `Send`-ness of the `check_outbound_to_agent` future; `held_backoff` dropping its mutex guard before `select!`; `run_link_pass` extraction in `graphs.rs`; `serde(default, skip_serializing)` on non-Option `f32`; `select!` in the gate loop.

## Not exercised
Held-call pacing; policy share/stage/adopt between two instances; held-search flow; raw-bytes snapshot on a real corrupt container; post-delivery link child on a real walk. Suite addition: `tools/third-party-verify/jurisdiction/j5-phase2.mjs` (syntax-checked only).
