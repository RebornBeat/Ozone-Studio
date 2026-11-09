# FINAL CAPTURE — 2026-11-08 (CC + ZCode, consolidated)

Status key: **BUILT+TESTED** (compiled, run live) · **EDITED** (committed, uncompiled/untested — operator gate) · **OPEN** · **WITHDRAWN**.
Branch `main`, 16 commits ahead of origin (282 files, +23,241/−2,017: docs 83, pipelines 91, host 53, tools 39, ui 12, shared 3). Not pushed.

## 1. Operator rulings (standing)
1. No confidence / false confidence anywhere; judgments are yes/no via strength-N votes; drops need unanimity; UI shows "Verified yes/no".
2. No fixed limits or caps (only explicit host-memory guards). Never drop zero-shot calls.
3. Hotness = OBSERVED USE only (`packed ÷ returned_traced`). **Stored hotness is trusted and read as is** (no recompute).
4. Stoplists only as a tested baseline, off by default (`OZONE_KEYWORD_STOPLIST=1`). Not rarity-first. Hits feed ML; zero-shot expansion last.
5. ML = pipelines (hashed-ngram encoder + pairwise ranker in pure Rust; pretrained/embedder slots prepared, honest refusal). MCP only for an external heavy trainer.
6. Hit/judgment logs are never rotated or truncated.
7. **No paid fallback** — the user-defined fallback chain (local BitNet / free / paid as configured) is untouched. **No quota sharing/limits/gating** — "it runs out as it runs out".
8. **Training never stops**: continuous, warm-started, low requirements; promotion gates *serving* only.
9. Broken/unbuilt pipelines (auth, sync, 18 unbuilt) get no action. Never `pkill -f`/`pgrep -f` own text.
10. Local first, then sync EU; no mocks; Ozone does work natively via /orchestrate.

## 2. Insights — status
| # | Insight | Status |
|---|---|---|
| 1 | Stage 12 failed "missing field 'action'": host passes `{data, context}`; mains must unwrap `data` (22 mains + traversal_ml fixed; ~67 audited) | BUILT+TESTED (Stage 12 live); remaining mains audited-by-list |
| 2 | Watchdog: flat 300 s killed `context_aggregation` → progress-aware (`progress_bounded`, model/store activity within 300 s, hard ceiling 14400 s) | BUILT+TESTED |
| 3 | Span-out leak into decoy projects → typed `Context.scope` derived from markers/ORIGIN | BUILT+TESTED (SW1c PASS ×2 binaries) |
| 4 | Substring flood in keyword search → `token` strategy, ranked v0 key | BUILT+TESTED (K1/K2) |
| 5 | Stage 4 cost = sequential candidate fetch round trips; write phase must stay sequential | explained; concurrent fetch waves BUILT+TESTED |
| 6 | Stage 3 latency (relevance-filter sequential votes, 448–523 s) | **OPEN** — operator rejected quota-based grouping; no ruling on concurrency / propose-then-confirm |
| 7 | Keeper pass quality works stoplist-free | BUILT+TESTED |
| 8 | Expansion over-adds (~10 terms/methodology) | OPEN (measurement needed on real hits) |
| 9 | Promotion math: needs ≥5 held-out groups all wins (sign test p<0.05) — gates serving only now | by design; training continues regardless (EDITED) |
| 10 | Quota economics: 65% of daily calls are own votes; background actors ignore exhaustion; settlement starves meta loop | quota-gating / paid-fallback advice **WITHDRAWN**; split loops EDITED |
| 11 | Invented hotness everywhere (239 literals + 51 expressions + text default 0.5 + consciousness emotional_significance) | all → 0.0 EDITED; legacy scrub EDITED |
| 12 | ZCode: walk 1158 AMT re-expansion duplication on finding 45238 competes for the rate budget while a prompt call streams 1022 s+ | OPEN (AMT 44 branches/one sentence; feasibility re-plan inflates plans) |
| 13 | ZCode: literal ruling reviewed vs production — no edits required; wave 2 deployed green (t9 257/257, K-suite 9/0/1) | BUILT+TESTED |
| 14 | ZCode: EU gates 0–4, uidocument/G12 | operator-gated; no action |
| 15 | OpenRouter free quota 1,011/1,000 used, resets 00:00 UTC; `limit_remaining` 48.58 credits, `usage_daily` $0 — no paid spend | fact; no action |

## 3. Why stored hotness is trusted (the operator's question)
Recomputing hotness at ranking time from counts re-derived a number the rollup had already derived, reviewed and stored — extra work and a second source of truth that could silently disagree. The only reason to distrust it was legacy invented values (node literals, consciousness `emotional_significance`). Those writers are now zero, and a one-time scrub resets any non-zero value on containers with no observed traced returns. After that, the stored hint is the single source of truth. Cost saved: one counts-lookup and division per candidate; correctness gained: one definition. EDITED, uncompiled.

## 4. Edit inventory by wave
- F1–F7 (earlier): envelope unwrap, progress watchdog, typed scope + backfill, span-out scoping, token search, concurrent fetch waves — BUILT+TESTED.
- Keyword waves 1–4: `shared/contracts/keyword_model.rs`, `src/keyword_program.rs`, `keyword_settle.rs`, hit/judgment logs, Origin propagation (task-local, env, headers), dataset builder, traversal_ml model actions, K1–K9 E2E — waves 1–2 BUILT+TESTED; 3–4 EDITED.
- Fifth wave (b785e0d): warm-started continuous training (`train_continuously`, rolling candidates), `keyword_loops.rs` housekeeping + settlement as own actors, `returned_traced`, 51 hotness expressions → 0.0, biology reads `enriched=true`, keyword hygiene — EDITED.
- Trust-stored-hotness (0167d8f): `facts_from_context(stored_hotness)`, search.rs passes hint, consciousness store 0.0, `scrub_legacy_hotness_once` — EDITED.
- Literal removal: 596 records in `docs/evidence/literal-removals/2026-11-08-removed-literals.jsonl`.

## 5. Verification status
BUILT+TESTED: through ZCode wave 2 (lib 257/257, K-suite 9 pass/0 fail/1 skip, SW1c PASS).
**NOT compiled/tested:** fifth wave + trust-stored-hotness (host, traversal_ml, text, 16 hotness-edited modalities, methodology_create). Every touched Rust file was rustfmt-parse-checked only. No claim of correctness beyond that.

## 6. Operator queue
1. Unlock build+test: host, traversal_ml, text, modalities, methodology_create; lib tests; restart host (pid-kill); K-suite; one real walk after the 00:00 UTC quota reset (needed for `applied` events, creation pairs, edge-use stats).
2. DROP-gate strength floor at flash-1.
3. Stage 3 latency decision (concurrent votes vs propose-then-confirm).
4. EU measurement run (`tools/keyword-measure/measure.mjs`); pretrained-encoder weights/runtime.
5. Push (16 commits ahead).
6. 28 stale queued tasks; paused tasks 1148/1152/1154; MCP servers down.
7. Hotness baselines: all zero now — grows only from observed use.
