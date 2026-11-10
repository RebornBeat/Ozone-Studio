# Built-in blueprint bodies (19) — 2026-11-10 — NOTHING COMPILED OR RUN

**Finding:** the bodies were NOT missing — they were authored earlier and live UNTRACKED at `zsei_data/local/blueprints/bp_*.json` (also under `target/*/zsei_data/blueprints`). `bootstrap.rs::copy_blueprints` copies `assets/blueprints/` (absent), so a fresh data dir never received them, and the 19 legacy bodies also use an OLD step schema (`step_id/name/outputs`, no `step_index`) that the current `BlueprintStep` reader cannot load. So I did not invent plans: `convert.py` converts them LOSSLESSLY into the current body shape and writes `assets/blueprints/bp_*.json` (19 files). Everything not representable is kept under `legacy_step_fields` / `legacy_methodology_config`.

**Pipeline renumbering hazard (important):** the legacy designs use an older pipeline numbering. Run as-is, legacy `1`, `7`, `10`, `11`, `12`, `16`, `33`, `40` would now call Auth / LibraryTab / Voice / MethodologyFetch / MethodologyCreate / ZeroShotSimulation / Sync / ExperienceCategorization. Each step was remapped by (legacy id, action) to the pipeline that does the job today: 1→9, 12→21 ContextAggregation, 16→11 MethodologyFetch, 40→39 ConsciousnessDecisionGate, 7→4 ZSEIWrite, 33 register/codify→12 MethodologyCreate else 9; 100/101/105/9 unchanged. `methodology_ids` = asset id N → container `30000+N` (all exist on the EU except asset 34, no container).

| bp | steps | pipelines used | methodology container ids | status |
|---|---|---|---|---|
| 1 General Assistant | 6 | 9, 21 | 30001, 30002 | NEEDS-OPERATOR: step 2 legacy `dynamic` (per-modality) → 9 |
| 2 Code Review | 10 | 9, 11, 21, 101 | 30003–30006, 30010 | confirm: step 4 TriggerHook → 9 |
| 3 Documentation | 15 | 9, 11, 21, 100, 101 | 30001, 30002, 30011, 30012 | confirm: step 13 CreateFile → 9 |
| 4 Code Creation | 15 | 4, 9, 11, 21, 39, 100, 101 | 30003–30008, 30010 | confirm: CreateFile/TriggerHook → 9, IndexFiles → 4 |
| 6 Data Analysis | 15 | 9, 21, 100, 101, 105 | 30003, 30005, 30009, 30013 | NEEDS-OPERATOR: step 2 `dynamic`; confirm hook/file steps |
| 7 Math Proof | 14 | 9, 21, 101, 105 | 30009, 30014, 30015 | confirm: step 5 hook → 9 |
| 8 Modality Pipeline Onboarding | 4 | 101 | 30019, 30037, 30038, 30040 | procedural dev blueprint; 101 kept (graph actions) — confirm |
| 9 Jurisdiction Expansion | 4 | 9 | 30021, 30043 | asset methodology 34 has no container |
| 10 Methodology Creation | 3 | 9, 12 | 30032, 30036, 30043 | ok |
| 11 Linking Audit | 3 | 9, 101 | 30036–30038, 30041 | ok |
| 12 Cross-Process Retrieval | 3 | 101 | 30023, 30025, 30029, 30040 | ok |
| 13 Text Document Analysis | 4 | 9, 100 | 30019, 30035, 30038 | ok |
| 14 Incident Postmortem | 3 | 9, 12, 101 | 30028–30030, 30036, 30043 | ok |
| 15 Agent Onboarding | 3 | 9 | 30020, 30026, 30033, 30038, 30039, 30043 | ok |
| 16 Data Integrity Audit | 4 | 101 | 30023, 30036, 30037, 30043 | ok |
| 17 Fallback & Capture Verification | 3 | 9 | 30019, 30035, 30036, 30038, 30043 | ok |
| 18 TD-1 | 3 | 9 | 30021, 30035, 30036, 30043 | the capture-assertion mechanism does not exist yet (disclosed in its own text) |
| 19 TD-2 | 3 | 9 | 30035, 30036, 30043 | same |
| 20 TD-3 | 3 | 9 | 30035, 30036, 30038, 30043 | same |

Dropped-in-shape (kept verbatim under `legacy_step_fields`): per-step `name`, `inputs`, `outputs`, `config`, `conditional`, `conditional_next`, legacy `loop_config` (different schema from `LoopConfig`), `triggers_hooks`, `critical`, `comment`, `on_failure`. `depends_on` was derived from outputs→inputs (else previous step); `rule_refs` empty (none invented).

**Verification (no build):** all 19 parse as JSON; a python check of every step against the `BlueprintStep` fields/types (required: step_index, action, description, pipeline_id; defaults for the rest), contiguous step_index, depends_on only to earlier steps, every pipeline_id in the real registry, every methodology id a real EU container. NOT verified: serde deserialization by the compiled binary, behavior of any step. Not touched: the existing EU data (`zsei_data/blueprints/` has only index + walks; the next bootstrap copy is what would place these files; the 19 containers' `object_store_path` `blueprints/bp_*.json` resolves there).
