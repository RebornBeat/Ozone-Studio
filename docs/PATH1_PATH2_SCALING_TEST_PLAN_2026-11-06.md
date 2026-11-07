# Path1 / Path2 speed + scaling test plan (NOTES ONLY — nothing run)

Operator: Path1/Path2 work as meant to (model-size paths: context ≤ 8,192 ⇒ Path2 granular loops, else Path1 whole-chunk); "we need to test speed difference and scaling difference — just for notes". This is the plan to capture it when the build/test gate is open.
- **Fixed inputs** (same text for every run): 1 sentence · 1 paragraph (~100 words) · 10 paragraphs · 100 paragraphs · ~1 MB; plus a structured doc (headings/code block) and a multi-modality prompt.
- **Windows/paths** (pin the model per run; same request via `/orchestrate`, `consciousness_enabled:false`): 4,096 (Path2) · 8,192 (Path2 boundary) · 8,193 (Path1 boundary) · 32,000 (registered router) · 128,000 (active backend) · 1,000,000 (proven model). Force with `processing_path` where the window cannot be set by a real model.
- **Record per run:** wall time per stage (Stage 2 especially), model calls (host ledger + `pipeline_zero_shot_calls.jsonl`), tokens, retries/unusable, daily quota consumed, peak RSS, graph size produced (nodes/edges/chunks), whether any trim/`context_trims` or `window_unknown` was emitted, watchdog hits.
- **Scaling questions:** calls vs input size (linear? Path2 ∝ 1/window as designed?), wall vs calls (rate-limiter bound at 20 rpm?), graph equivalence between Path1 and Path2 (same entities/relations? differences documented), quota cost per KB.
- **Controls:** standard vs `high_end` (concurrent votes) on the same input; proven-in-slot on/off (`OZONE_PROVEN_FIRST`); both processing paths on a stable model.
- **Output:** a table `input × window → calls, wall, tokens, graph size, notes` in docs/evidence/test-runs/, and the per-size entries the W12 store would key on.
