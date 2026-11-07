# Walk completion guide — what "100%" means for a walk and for every stage (operator 2026-11-07)

Goal: **every walk completes (100%), every stage is reached and healthy (100%), every tracked keyword family is usable (100%)** — and all of it is measured, not asserted. Tool: `tools/walk-scorecard/scorecard.mjs` (reads `zsei_data/walk_metrics.jsonl` — one line per stage event WITH its walk `trace_id` — and falls back to the persistent host log for older walks). Output: `docs/evidence/walk-scorecards/<date>.md/.json`. Test: `tools/e2e/walk-completion.mjs` (MASTER_TEST_GUIDE §27).

## 1. The metric
**Walk completion % = completed CORE stages ÷ 13.** Core = 1 Input Capture, 0 Jurisdiction Gate, 2 Text Normalization, 3 Gather Methodologies, 4 Initial Graph Creation, 5 Build AMT, 6 Blueprint Assignment, 7 Zero-Shot Simulation, 14 Jurisdiction Full Gate, 9 Task Creation, 10 Step Execution, 11 Result Collection, 13 Response Delivery. Stages 8 (Consciousness Gate) and 12 (Post-execution Consciousness) are reported but not required (a walk with consciousness disabled legitimately skips them). **Funnel** = % of walks that REACH each stage — the lowest-reach core stage is where walks die. A walk is COMPLETE (100%), RUNNING (event < 15 min ago) or STOPPED before stage N.
**First measurement (14 walks since 2026-11-03, host log):** 1 complete (7%); reach: Stage 2 50% · 3 50% · 4 50% · 5 43% · 6 29% · 7 29% · 8 21% · 14 21% · 9 21% · 10 7% · 11 7% · 13 7%. Stage medians: Stage 2 20.2 min (p90 28.4), Stage 5 7.0 min (p90 19.5), Stage 6 58.7 s, Stage 7 39.1 s, Stage 8 45.3 s, Stage 10 14.7 min (one walk). 8 of the 14 died in Stage 2 under the old 1,800 s watchdog.

## 2. Per stage: what 100% means, what we track, what we found
| Stage | 100% means | Tracked per walk | Finding so far (evidence) | Fix status |
|---|---|---|---|---|
| 1 Input Capture | prompt captured | reached | always reached | — |
| 0 Jurisdiction Gate | gate evaluated, whole text seen (J8) | blocked? matched/warn counts | always passes; whole-text gate verified | — |
| 2 Text Normalization | chunks/graphs built, **usable keywords ≥3**, entities/topics extracted | duration, chunk count, `Keywords: usable of extracted`, calls, retries, reuse hit | median 20.2 min; K-algorithm ≥34 sequential calls/sentence; keywords came back as `["you", "<whole sentence>"]` | votes unified+configurable; streaming/progress timeouts; analysis reuse store; **keyword hygiene (L29)** — uncompiled |
| 3 Gather Methodologies | relevant methodologies found **by keyword** (floor NOT used), tools bound | methodologies, keywords usable, **floor used?** | bad keywords → floor loaded EVERY methodology (87→99, grows with the store) → 61 AMT branches, 12 steps, 9 invented "missing" | **relevance-first floor (L29)** — uncompiled |
| 4 Initial Graph Creation | modality graphs created, cross-modal links attempted once per pair | graphs, cross-modal count, passes | one walk spent ≈14 min here; the cross-modal loop never settled (31-pass ceiling) | once-per-pair (L21) — verified compiled, not exercised |
| 5 Build AMT | AMT intents/branches proportional to the REQUEST | branches, details, mode | 61–175 branches for a one-line request | follows from Stage 3 fix |
| 6 Blueprint Assignment | blueprint with **0 unresolved missing capabilities** | steps, missing, replans | 9 "missing" were invented needs; provider step planned for an inapplicable rule | **capability closure (L28)** — uncompiled; rule-condition check still open |
| 7 Zero-Shot Simulation | feasibility ≥ medium | feasibility, clarifications | "low" in 1/4 walks | closure raises it; post-sim re-plan open |
| 8 Consciousness Gate | decision Proceed with reasons | decision | Proceed (gate-reported) | — |
| 14 Jurisdiction Full Gate | resolved-plan re-check passes | blocked? | passes | — |
| 9 Task Creation | task recorded | task id | reached by 21% | — |
| 10 Step Execution | **all steps complete** (steps executed = planned) | steps executed/planned, failures, pauses | 3 step-failure PAUSEs; one walk ran 14.7 min; provider step failed ("none of the 3 tools matches") | failure doctrine pauses+resumes; tool choice from step text |
| 11 Result Collection | all step results collected | count | — | — |
| 12 Post-execution Consciousness | experience stored | — | — | — |
| 13 Response Delivery | answer delivered | duration | one walk took 7.5 min | — |

## 3. Keywords we track (families, where produced, metric, target = 100%)
| Family | Produced at | Metric | Target | Status |
|---|---|---|---|---|
| **Request keywords** | Stage 2 (model extraction) → `state.keywords` | usable ≥3 after hygiene; stopword rate; whole-sentence rate | 100% of walks ≥3 usable | hygiene added (L29); `Keywords: N usable of M extracted` in the Stage 2/3 summary and `walk_metrics` |
| Topics / entities | Stage 2 | non-empty for non-trivial prompts | 100% | tracked via summary (`Modalities`, `SentenceNodes`) — extraction success per call in `pipeline_zero_shot_calls.jsonl` |
| **Search keywords → methodology matches** | Stage 3 `search_by_keywords(Methodology)` | matches found by keyword (not floor) | floor used 0% | `Floor: yes/no` per walk; relevance-first floor |
| Methodology keywords | methodology containers (create/update) | every methodology has ≥3 real keywords | 100% | open (audit the 99 methodologies; meta-drafted ones may be thin) |
| Capability-gap keywords | `record_capability_gaps` ("capability" + name words) | each gap resolved: not_needed / methodology_drafted / satisfied / wanted | 0 unresolved | status tracked in `capability_gaps.json` (L28) |
| Scope keywords (ripple) | container writes (`proj:`/`file:` keywords) | n/a (structural) | — | by design, never forged (sanitized) |
| Marker vocabulary | `emit_marker` | 15/15 codes have an emit site | 100% | 15/15 after L26 |

## 4. How to track (code/harness)
`walk_metrics.jsonl` (host, one write per stage event) → `scorecard.mjs` (completion %, funnel, per-stage medians, per-walk keyword/floor/branches/missing/feasibility columns, findings) → `walk-completion.mjs` (asserts the criteria in §2 on a real walk) → evidence files. To raise a number, find its row in §2 and its test in §27; do not change the metric to fit the result.
