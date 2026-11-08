# Literal removal — invented hotness / edge weight / confidence (2026-11-08)

**Operator ruling (2026-11-08):** lift the standing "modality literals untouched" instruction. `hotness_score` is **kept and READ for ranking**, derived from real events; the invented **confidence** literals are removed; **edge weights** are decided on what the system actually has. Before removal everything removed was captured: `docs/evidence/literal-removals/2026-11-08-removed-literals.jsonl` (545 records: file, HEAD line number, field, old value, new value, enclosing type, reason, the source line). It is regenerable from git (`git show HEAD:<file>`) with the pass script's `manifest` mode.

## What changed (production code only — test fixtures were left alone and are not in the manifest)
| Kind | Count | Was | Is | Why / how it is scored now |
|---|---|---|---|---|
| Node `hotness_score:` literals (18 modality graph builders) | 239 | 0.5–0.9 invented | **0.0** (= no observed use) | `hotness` is now OBSERVED: `packed ÷ returned` from the real hit log, rolled up per container (`keyword_meta["*"]`, mirrored to `hints.hotness_score`) and **read by the ranking** (`RankFacts.hotness` in the v0 key and as a ranker feature). Ranking never reads the stored `hints.hotness_score` (legacy rows may still hold an invented literal) — it recomputes from the counts. |
| Edge `weight:` literals on `*Edge`/relation structs | 295 | 0.6–0.95 graded invented | **1.0** (= relationship present) | "relationships present or absent". The 226 literals that were already 1.0 stay. Real use of each relation type is now CAPTURED as counts (`zsei_data/edge_use_stats.json`: how many context windows were filled through `Calls`, `SimilarTo`, `child`, …); span-out keeps its structural class priors (`relation_weight`) for expansion order until the operator rules on switching to the observed order. |
| `confidence:` graded literals (eeg ×4 incl. one FABRICATED `0.85 + rand_float()*0.1`, dna ×3, math ×2, voice, task_recommendation) | 11 | 0.75–0.95 | **0.0** (= not affirmed) | the convention already used in the code is 1.0 affirmed / 0.0 not affirmed; no evidence-backed affirmation exists for these values, so none is claimed. Only strength-N yes/no judgments affirm anything. |
| `zero_shot_simulation` (general pipeline) | 5 sites | asked the MODEL for `confidence` 0–1 (+ fabricated defaults 0.7/0.85/0.4) and used `confidence >= 0.7` as the methodology verdict | the model is no longer asked; `confidence` fields are `0.0` (retired, wire-compatible); **`methodology_valid` = the yes/no `feasible` verdict** | self-reported model confidence is exactly the false-confidence class. |
| `biology` "enriched pathways" query | 1 | `hotness_score > 0.8` (the only consumer of the invented values) | `hotness_score > 0.0` (observed use only) | returns nothing until real events exist — honest. |
Most frequent removed values: weight 0.8×110, weight 0.9×97, hotness_score 0.7×56, hotness_score 0.65×35, weight 0.7×35, hotness_score 0.75×34, hotness_score 0.8×29, hotness_score 0.6×29, weight 0.85×29, hotness_score 1.0×18.

## Per-file counts
| Modality / pipeline | hotness | weight | confidence |
|---|---|---|---|
| `general/task_recommendation` | 0 | 0 | 1 |
| `general/voice` | 0 | 0 | 1 |
| `modalities/3D` | 23 | 10 | 0 |
| `modalities/BCI` | 13 | 13 | 0 |
| `modalities/CAD` | 14 | 14 | 0 |
| `modalities/IMU` | 12 | 13 | 0 |
| `modalities/biology` | 16 | 30 | 0 |
| `modalities/control` | 15 | 23 | 0 |
| `modalities/depth` | 8 | 13 | 0 |
| `modalities/dna` | 0 | 0 | 3 |
| `modalities/eeg` | 0 | 0 | 4 |
| `modalities/electromagnetic` | 9 | 14 | 0 |
| `modalities/geospatial` | 11 | 18 | 0 |
| `modalities/haptic` | 16 | 27 | 0 |
| `modalities/hyperspectral` | 7 | 17 | 0 |
| `modalities/kinematics` | 16 | 17 | 0 |
| `modalities/math` | 0 | 0 | 2 |
| `modalities/network` | 14 | 19 | 0 |
| `modalities/proteomics` | 14 | 16 | 0 |
| `modalities/radar` | 11 | 3 | 0 |
| `modalities/sonar` | 14 | 9 | 0 |
| `modalities/sound` | 16 | 19 | 0 |
| `modalities/thermal` | 10 | 20 | 0 |

## What is NOT changed (and why)
* Test fixtures/inputs (in `#[cfg(test)]` blocks) — not production data.
* The 226 `weight: 1.0` literals and the existing `confidence: 1.0` / `0.0` literals — already the yes/no convention.
* `relation_weight` / `STRUCTURAL_*_WEIGHT` in `span_out.rs` — expansion-ORDER priors by relation class (no edge-confidence term), kept; the observed counts that could replace them are now captured.
* **Already-written graph FILES and containers keep their old invented values** until the content is re-analysed (a regeneration writes the new values). The ranking is unaffected (it reads observed counts), the biology filter and the UI edge weight display will show old values on old graphs.
* `hints.hotness_score` on containers other than rolled-up ones (e.g. consciousness experience containers set it from emotional significance) — different semantics, untouched.
