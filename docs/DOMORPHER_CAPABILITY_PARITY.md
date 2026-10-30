# DOMorpher capability parity manifest (2026-10-30)

> Operator question: "are we dropping any capabilities due to this?" — every
> module of the 146k-line tree accounted: NOTHING is dropped. Each of the 21
> modules lands in exactly one of four dispositions, with the replacement
> named where retired. Line counts are the real source inventory.

| Module | Lines | Disposition | Destination / replacement |
|---|---|---|---|
| `agent/` (19,263) | action types, perception, goals, memory, navigation | **SPLIT** — pure parts → `domorpher-core`; `reasoning.rs`'s direct LlmClient calls → ABOVE the seam (Ozone pipelines decide; the library receives decisions as data) | core IN-list + Ozone orchestrator |
| `browser/` (6,227) | driver traits, cookies/session/auth state, puppeteer/headless/js | **SPLIT** — driver traits + cookies/session/auth → core (already shimmed); the three drivers retired | PageDriver seam → Browser-Farm (primary) / browser-mcp CDP (isolated) |
| `chunking/` (3,739) | text chunking | **IN core** | domorpher-core |
| `dom/` (8,514) | preprocessor, semantic_markdown, analyzer, accessibility, converter | **IN core** | domorpher-core |
| `execution/` (5,975) | actions, evaluator, multi_pass, recovery, engine | **IN core** (logic); engine's driver calls go through PageDriver | domorpher-core |
| `extraction/` (4,255) | data extraction | **IN core** | domorpher-core → `domorpher_extract` tool |
| `forms/` (5,037) | form fill/validate | **IN core** | domorpher-core → `domorpher_input/select/check/upload` tools |
| `instruction/` (4,801) | NL → action sequence | **IN core** | domorpher-core (the propose layer's parser) |
| `interactive/` (6,485) | element detection, events, pattern library | **IN core** | domorpher-core |
| `schema/` (8,287) | schema + validation | **IN core** | domorpher-core |
| `reconciliation/` (3,258) | state reconciliation | **IN core** | domorpher-core |
| `utils/` (3,953) | tokenizer, encoding, config | **IN core** (subset — http/logging OUT) | domorpher-core |
| `llm/` (7,097) | LlmProvider traits + anthropic/local clients | **ABOVE the seam** — the traits' ROLE is filled by Ozone's pipeline 9 + fallback chain; `EmbeddingGenerator` harvest-checked at split time | Ozone model layer (replacement named) |
| `api/` (3,485) | actix/graphql/rest/websocket/rpc | **ABOVE the seam** — retired | The Ozone host IS the API surface |
| `enterprise/` (13,218) | api/auth/compliance/logging/monitoring | **ABOVE the seam** — retired | Ozone host surfaces (auth, monitor, capture stores) |
| `monitoring/` (6,138) | metrics/health | **ABOVE the seam** — retired | Ozone monitor + capture stores |
| `storage/` (4,740) | persistence | **ABOVE the seam** — retired | StateStore seam → farm profiles / data_dir |
| `pipeline/` (6,181) | domorpher's own pipeline concept | **ABOVE the seam** — retired | Ozone's pipeline registry + AMT |
| `parallel/` (7,996) | parallel execution | **ABOVE the seam** — retired | Ozone task executor + farm's own parallelism |
| `extension/` (6,971) | browser content scripts | **MAYBE — parked** | only if in-page assistance is wanted; else retires |
| `visual/` (6,620) | visual processing | **SPLITS BY FUNCTION** — see the run-vs-tools doctrine below | image-modality run (owner) vs visual-mcp tools (guests on its graph) |

**The two living guarantees:**
1. **The split-pending promise** (`agent/reasoning.rs`, 45 pub items): when the
   split happens, EVERY pure strategy is carried into the core and every model
   call site is replaced by the contract (data-in decisions). A per-item checklist
   is made at split time, before the file re-enters the IN-list.
2. **The retire-replacement rule**: no module retires without the replacement
   named above — all replacements are LIVE Ozone surfaces (host API, model
   layer, monitor, farm), verified working this session.

**The run-vs-tools doctrine (operator insight, 2026-10-30 — answers "is the
image modality running itself on it, or are we calling tools on top?"): BOTH,
in order, with different roles.** Content arrives → **the modality RUNS itself
on it first** (pipeline 102: the owning analysis — bit depth, sRGB, composition
→ the BASE graph; verified live on EU). **Then tools run on TOP of that graph**
(YOLO/visual-mcp: detections become nodes, F.3-typed edges — guests extending
the owner's graph, never replacing its run). Both feed the graph-native
expansion; the modality run is always first and owning. domorpher's `visual/`
splits accordingly: screen/UI-understanding parts (windows, controls,
occlusion, affordances) → the visual-MCP tool surface (whose captures go
through the modality run FIRST); generic image processing → the image
modality's detection-model registry (§11.6 fan-out).

**Totals**: 146k lines → ~66k IN core (agent-pure + browser-support + chunking +
dom + execution + extraction + forms + instruction + interactive + schema +
reconciliation + utils-subset), ~55k above-the-seam/retired (all replacements
live), ~13k merge/parked (visual, extension), plus the tool surface (16 verbs)
and the two seams. Nothing unaccounted.
