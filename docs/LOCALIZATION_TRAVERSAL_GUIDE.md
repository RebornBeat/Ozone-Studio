# Localization Traversal — generalizing the detection-model-registry pattern across every spatial modality

> Operator framing, 2026-09-29: every modality with a real spatial referent
> — 3D, CAD, and beyond — decomposes into the same move: **localize**
> (place real entities in space) **then traverse** (walk the resulting
> structure). §11.6 (`docs/TOOLS_PIPELINES_MCP_GUIDE.md`) already proved
> this pattern once, for real, on image 102: a registered detection model
> (YOLO) localizes objects, and the graph traversal that follows is what
> turns detections into a living, queryable structure (container 40277 —
> real person nodes + a real `Overlaps` edge from actual bounding-box
> geometry). This guide generalizes that one proof into a doctrine: the
> same registry-of-localization-models pattern, applied to every modality
> whose real-world referent has spatial structure, is a mechanical,
> roughly one-day-per-modality build once the recipe is followed — not a
> from-scratch design problem for each one.

Back to [`TOOLS_PIPELINES_MCP_GUIDE.md` §11.6](TOOLS_PIPELINES_MCP_GUIDE.md)
(the proven pattern this generalizes) · [`NEW_MCP_GUIDE.md`](NEW_MCP_GUIDE.md)
(Pattern A/B/C, the umbrella recipe) · [`REVERSE_ENGINEERING_GUIDE.md`](REVERSE_ENGINEERING_GUIDE.md)
(the sibling doctrine for unknown-software observation, same "structure
before intelligence" spirit).

---

## 1. The principle, precisely

§11.6 already established this shape for image 102, real and running:

1. A **registry** of localization models — data, not code (`zsei_data/detection_models/registry.json`: model file + class list + input size per entry). Adding a model is adding a JSON entry.
2. Each model runs **as a tool**, callable through `/mcp/call`, metered/gated/rippled like everything else (`yolo_detect`, `yolo_graph`).
3. A model's output — real detected entities with real spatial coordinates — becomes **real graph nodes**, and the spatial relationships between them (computed from the actual geometry, never guessed) become **real typed edges** (F.3: `Overlaps`/`Above`/`Below`/`LeftOf`/`RightOf` for image; the 3D/CAD equivalents are the same idea in one more dimension).
4. N models can run **in parallel**, all expanding the same graph natively — this is the edge-identification family's parallel fan-out pattern, reused, not reinvented.

**The generalization**: this is not an image-specific trick. Any modality whose real-world referent has a spatial layout — a scene, an assembly, a terrain, a sensor array, a molecule — can use the identical shape: register a real localization model for that domain, run it as a tool, turn its output into real nodes + real geometrically-derived edges. What differs per modality is only *which* real models exist for that domain and *which* spatial-relation vocabulary is correct for it (F.3 for 2D image; a 3D analog for scenes/CAD; sequence/topology-aware relations for EEG's sensor array; sensor-network relations for radar/sonar; etc.) — never the pattern itself.

## 2. Real modality inventory, checked directly (not assumed)

27 real modality source directories exist
(`assets/pipelines/modalities/`). Verified state as of this guide, not
carried over from an earlier pass:

| Modality | Registered in pipeline index? | Compiles? | Spatial/localization fit |
|---|---|---|---|
| **image (102)** | yes | yes, revived this session | Proven — §11.6, container 40277 |
| **3D** | yes, as `3d_engine` | yes, revived this session | Proven — container 40232, real Blender-scene test |
| **CAD** | **no** — not in `zsei_data/pipelines/index.json` at all | **yes, as of 2026-09-29** — the 4 compile errors below were fixed same-session; persistence/registration are the real remaining layer-1 work | Strongest near-term candidate: `main.rs` (108KB) already has real intended structure — `CADGraphQuery::InterfacingFaces { part_a_id, part_b_id }`, `CADHeadlessOp::GenerateBOM { output_path }` are real enum variants already written, not proposed. The 4 real compile errors that blocked it are now fixed (3× `E0408` or-pattern binding mismatch in the `AnalyzeFile`/`AnalyzePart`/`AnalyzeAssembly`/`AnalyzeDrawing` match — the unused `extract_*` bindings were dropped, never read anywhere in the arm; 1× `E0277`, a `Vec<&String>` `Display` error in a GD&T-frame content string, fixed with `.join(", ")`). Real binary now builds (`target/release/CAD`, exit 0). Still needs: ZSEI persistence wiring, `zsei_data/pipelines/index.json` registration, and only then a layer-2 localization-model registry — the mechanical revival recipe's remaining steps. |
| **depth** | **no** — not registered at all, earliest-stage of any of these | not checked this pass | The real technical bridge the operator described: a depth-estimation model turns a single 2D image into per-pixel spatial data — the "flash a 2D read onto a 3D structure, even hollow/approximate, then refine" route. Real, existing open-source model families for this: MiDaS, Depth-Anything (both real, commonly-used HuggingFace models — named here as known real options, not vetted/selected). |
| **geospatial** | yes | not checked this pass | Explicitly spatial (mapping/terrain) — a natural fit once revived. |
| **chemistry / proteomics** | yes | not checked this pass | Real 3D molecular structure — localization here means atomic/residue positions; a different model family (molecular geometry / structure-prediction models) than image-style detectors, but the same registry-of-models → nodes+edges shape. |
| **eeg / BCI** | yes / not confirmed | not checked this pass | Spatial in a different sense — a real sensor-array topology, not a 3D scene. Localization here likely means electrode-position-aware signal source localization, a real, distinct model family from image/3D detectors. |
| **IMU / kinematics / haptic / radar / sonar / electromagnetic / thermal / hyperspectral / control / biology / dna / network** | yes (per ZCode's earlier "27 stubs" accounting) | not checked this pass | Each has *some* real spatial or positional referent (motion, contact points, sensor returns, temperature-over-space, wavelength-over-space) — worth the same one-day-per-modality mechanical pass, but not individually scoped in this guide; flagged as the long tail, not pre-designed here (per this doctrine's own "identify, don't hand-design every one up front" spirit — the recipe should make each one fast to scope when its turn comes, not require a bespoke plan now).

## 3. The generalized build recipe (one day per modality, per the operator's own estimate)

Two layers, in order — do not skip to layer 2 before layer 1 is real for a
given modality:

**Layer 1 — mechanical revival** (the recipe this session already proved five
times over: image, 3D, audio, video, plus the earlier text/code/math
baseline). For CAD/depth/geospatial/etc., in order:
1. Fix the CLI contract (`--input`/stdin + `{data, context}` envelope unwrap — the exact class of bug that blocked every modality before this session's revivals).
2. Fix whatever real compile errors exist — **CAD's own 4 errors are already fixed, same-session as this guide** (3× E0408 or-pattern binding mismatch, dropped bindings nothing downstream read; 1× E0277 Display error, fixed with `.join(", ")` to match every other modality's clean `content`-string convention). Real `cargo build --release` exit 0 confirmed.
3. Wire real ZSEI persistence (`CreateContainer`, parent = project_id, content at `graphs/{modality}_<id>.json` — the exact pattern `persist_zsei()` already established for image/3D/audio/video). **Not done for CAD yet** — next real step.
4. Register in `zsei_data/pipelines/index.json` if not already present (CAD and depth both need this step; most of the other 27 already have it).

**Layer 2 — the localization-model registry** (§11.6's real pattern, applied
fresh per modality):
1. A `zsei_data/detection_models/registry.json`-style entry set for the modality's own real model family (depth-estimation models for `depth`; a 3D/point-cloud detector family for `3D`/`CAD`; whatever the domain's real equivalent is — never invent a model family without a real, named, existing option, per this project's own no-fabrication doctrine).
2. One or more real tools (`<model>_detect`, `<model>_graph` — mirroring `yolo_detect`/`yolo_graph` exactly) through `/mcp/call`.
3. Real spatial-relation computation from the model's actual output geometry — the modality's own equivalent of F.3, not a copy-paste of image's exact edge types where they don't semantically apply.

## 4. The specific technical direction the operator named: image-gen/depth as the 2D→3D bootstrap

The fastest real route into 3D localization for a scene that only has 2D
image data available: a depth-estimation (or related image-generation)
model produces a spatial read from a single 2D image — even an
approximate, "hollow" top-down structure is real, useful spatial data,
not nothing — and that becomes the starting point a further pass
refines, rather than requiring true 3D sensor data before any spatial
graph work can begin at all. This is the same "detection models are tools
latent to a modality, not a monolithic analyzer" framing §11.6 already
established for YOLO — extended here to depth/image-generation models as
a second, complementary tool family alongside pure object detectors,
both registered the same way, both expanding the same graph.

**Status, 2026-09-29: built and live-verified.** `depth_estimate` +
`depth_graph` tools landed in `tools/visual-mcp/server.mjs`, registered
in `zsei_data/detection_models/registry.json` (v0.2.0, real `kind` field
distinguishing detection/depth models in one shared registry) and in the
real `/mcp/tools` registry. Model: `depth-anything/Depth-Anything-V2-Small-hf`,
real inference verified on `zidane.jpg`. `depth_graph` persists real
per-object depth as a node attribute and computes real `InFrontOf`
relations from actual relative-depth values — verified twice: directly
against visual-mcp, and through the full gated `/mcp/call` stack
(jurisdiction/ripple/S13/identity, identical to the proven YOLO path).
Full writeup: `CHECKLIST.md`'s 2026-09-29 "DEPTH ESTIMATION MCP LIVE" entry.

**Still open** (real, not decided here):
- `InFrontOf` is computed and returned but **not yet a persisted typed
  graph edge** — `image 102`'s `ImageEdgeType` enum has no depth-axis
  variant today. Adding one is a real Rust schema change + rebuild,
  deliberately not done in the same pass as the Python/Node tool work.
- Whether "flash onto a 3D wiring top-down" ever becomes a literal new
  `ThreeDNodeType`/edge convention, or reuses/extends the existing 3D
  modality's real types, is still unopened — 3D's own real
  `ThreeDNodeType`/spatial-relation types (from this session's 3D
  revival) should be checked first before adding new ones.
- A pre-existing, unrelated anomaly was found (not fixed): the persisted
  graph's root Image node reports `800x600` when the real file is
  `1280x720` — the same `Analyze` code path `yolo_graph` already used
  before this work, so not introduced here, just newly visible.

## 5. Audio/voice: Whisper as reference, not a fixed requirement

The operator's original intent was to route voice through Whisper into
the audio-modality graph-expansion pattern above. Explicitly stated as
open, not fixed: Whisper is a real, usable reference/baseline, but if its
compatibility or quality falls short once actually tried, the operator
wants something more powerful built to replace it outright — not a
Whisper-shaped constraint the audio MCP is locked into. Whoever builds
the audio MCP (transcribe/speak → audio 103 graph expansion, per ZCode's
own earlier gap-scan item) should treat "is Whisper actually good enough
here" as a real, open technical evaluation, not an assumed yes.

## 6. Full missing-tools/MCPs inventory (merges ZCode's own capability-gap scan with this guide's additions)

**From ZCode's capability-gap scan** (2026-09-29, operator-requested), preserved here for one combined list:
1. Web-search MCP wrapping the built Brave pipeline 56 (needs `BRAVE_SEARCH_API_KEY` only).
2. Browser MCP wrapping the built pipeline 25 — pairs with visual+YOLO for the `REVERSE_ENGINEERING_GUIDE.md` visual-surface collector. ZCode's open question (browser automation surface vs. capture→graph wiring split) is still open — not decided in this guide; a separate, smaller-scope conversation.
3. File-beacon — **done, this session** (`src/file_beacon.rs`, live-verified).
4. Git MCP (typed, safer than raw exec through terminal-mcp).
5. Audio MCP — see §5 above for the real, undecided Whisper question.
6. Detection-model expansion for image 102 — registry is data, no code needed (already true today, per §11.6's own design).
7. Keep-warm llama MCP (kills BitNet's 25-40s cold-load per call).

**Added by this guide** (the localization-traversal generalization):
8. CAD revival (layer 1) — 4 real compile errors already identified as the concrete starting point, real intended structure (`InterfacingFaces`, `GenerateBOM`) already present in source.
9. Depth-estimation tool/registry (layer 2, image-adjacent) — the 2D→3D bootstrap route; register a real model (MiDaS/Depth-Anything) the same way `yolo_detect` was registered.
10. 3D/CAD localization-model registry (layer 2) — once 3D (already revived) and CAD (layer 1 first) both have real persistence, give them their own §11.6-style model registry rather than treating image as the only modality with this capability.
11. The remaining spatial-adjacent modalities (geospatial, chemistry/proteomics, eeg/BCI, and the longer tail in §2's table) — flagged as the long tail this recipe makes fast to pick up one at a time, not pre-scoped individually here.

---

Sent to ZCode via the shared-context coordination channel alongside this
guide's write-up — see the linked handoff note for the specific proposed
split of who builds what next.
