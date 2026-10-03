# Capability Expansion Review — MCPs, modalities, cross-capabilities

> Doctrine doc, 2026-10-02. Operator directive, verbatim intent: expand
> MCPs/tools; stress-test modalities further (image/3D/CAD framed as
> having real "foundations" — lines, shapes, 2D/3D geometry, the way
> `yolo_detect`/`depth_estimate` already ground image); review-only
> first, in forks, before any building; also expand cross-modal
> capability. This doc is the synthesis of 4 parallel fork reviews
> (image/2D, 3D/CAD, the other 23 modalities, cross-modal fusion) — real
> findings only, every candidate named concretely, nothing vague. No
> code was written producing this doc; it is the review the operator
> asked for, not the build.

---

## 0. The most important finding first — an active integrity violation, not a gap

**`detect_text` (OCR) and `detect_faces` in
`assets/pipelines/modalities/image/main.rs:1253-1286` are 100% hardcoded
fabrication**, confirmed live: `detect_text` always returns a fake
`"Sample text"` region at fixed coordinates regardless of real image
content, on **every** real `visual_describe`/`visual_ingest`/
`visual_graph` call (`tools/visual-mcp/server.mjs` hardcodes
`detect_text: true`). `detect_faces` is equally fake but not currently
triggered. This is shipping today, not theoretical, and directly
violates this project's own no-fabrication doctrine. The code's own
comment already names the fix: `// OCR would go here (Tesseract,
EasyOCR, etc.)`. **This should be fixed before anything else in this
doc** — it is a correctness bug wearing a "missing feature" disguise.

---

## 1. Image / 2D-visual modality

### What's real today (re-verified, some session notes were stale)
- `tools/visual-mcp/server.mjs`, 7 tools, confirmed live: `visual_describe/ingest/graph` (real object/color/composition analysis; text/faces fake per §0), `yolo_detect/yolo_graph` (real — **ultralytics 8.4.165 is now installed**, closing the "needs pip install" note from earlier this session), `depth_estimate/depth_graph` (real, Depth-Anything-V2-Small).
- `zsei_data/detection_models/registry.json` v0.2.0 — the real "add a JSON entry, N models run in parallel" pattern, proven with yolov8n + depth-anything-v2-small.
- **OpenCV 5.0.0 is already installed** in the same venv, unused by anything today.
- Real schema gap: `DetectedObject.mask: Option<SegmentationMask>` exists but is always `null`. `ImageEdgeType` has no `NearTo`/`Supports`/depth-axis variant — the MCP computes these relations but only `Above/Below/LeftOf/RightOf` actually persist as real graph edges.

### Real, concrete candidates (ranked)
1. **OCR fix — `pytesseract` + Tesseract.** Fixes §0 and is the natural lightweight-OCR answer simultaneously. CPU-only, zero GPU. **Highest priority in this whole document.**
2. **Classical edge/line/shape detection — OpenCV's own `Canny`/`HoughLinesP`/`findContours`.** Zero new dependency (already installed), zero model weight, deterministic — matches the operator's own "lines, shapes" framing exactly, and is the cheapest, fastest, most honest option reviewed anywhere in this doc.
3. **Pose estimation — MediaPipe Pose** (30+ FPS CPU) or **MoveNet Lightning** (<7ms/frame). Either slots into the registry as `kind:"pose"`.
4. **Segmentation — don't add a new dependency.** ultralytics (already installed) ships YOLOv8-seg/YOLOv9-seg checkpoints directly; add a `kind:"segmentation"` registry entry with a `-seg` weight file. MobileSAM is the real alternative if ultralytics' own segmentation proves insufficient, but it's ~300ms/image on CPU — meaningfully slower.
5. **Whole-image classification — MobileNetV3** (~6MB) or **EfficientNet-Lite0** (4.7M params, 6.5ms INT8 CPU).

Everything in 2-5 fits the existing detection-model-registry pattern unchanged (new `kind`, new JSON entry, one new invocation script mirroring `runYolo`/`runDepth`'s shape). Only #1 is structurally different — it patches the existing `Analyze` action rather than adding a tool.

---

## 2. 3D / CAD modality

### What's real today
- Two distinct real modalities (3D=109, CAD=120), each with a substantial real Rust pipeline (`assets/pipelines/modalities/3D/main.rs` 2533 lines, `.../CAD/main.rs` 2091 lines) — real deep schemas (`MeshInfo`, `BoundingBox3D`; `FeatureInfo`, `HoleType`, `SketchInfo`, `PartInfo`/`AssemblyInfo`), real declared file-format support (GLTF/OBJ/FBX/STL/PLY for 3D; STEP/IGES/SLDPRT/FCStd for CAD). The operator's "they have foundations" is accurate, not aspirational.
- `tools/threed-mcp/` is narrow (one Blender-headless asset generator, documented hang + working timeout). `tools/bridges/` (Blender/Unity/Unreal/Roblox) is the real, live general 3D-scene surface.
- `docs/MODALITY_EXPANSION_GUIDE.md` already names 46 candidates across all modalities including a real 3D/CAD physical-bridge roadmap (COLMAP, Open3D, ORB-SLAM3, FreeCAD headless) — this section is a **lightweight-axis addendum** to that doc, not a duplicate; that doc never assessed resource weight.

### Real, concrete candidates — NOT in the existing guide
1. **trimesh** (pure Python, numpy-core) — loads STL/OBJ/PLY/GLTF/3MF, has `bounding_box.extents`/`convex_hull` built in. Direct match for "lines shapes 2d 3d shapes." Fits as a **new lightweight MCP tool** (`mesh_analyze`) — works on a bare file, no live engine needed (unlike bridges).
2. **numpy-stl** — STL-specific, vectorized surface-area/volume/transform. Even lighter than #1, narrower scope.
3. **meshio** — numpy-only, broad format conversion (OBJ/PLY/STL/VTK/Gmsh) — the natural front-end normalizing any mesh format before #1/#2 touch it.
4. **python-step-parser** (pure Python, only needs `sqlite3`, actively maintained) — reads real STEP/STP structure/metadata **without any CAD kernel** — genuinely lighter than the existing guide's FreeCAD-headless recommendation for this specific case.
5. **cascadio** — a minimal OpenCASCADE binding, loads BRep-family CAD straight into a triangulated mesh, skipping a full FreeCAD install when the real need is "get a measurable mesh," not parametric editing.
6. **point-cloud-registration** + **pyntcloud** (pure Python, numpy-only) — real, lighter alternatives to Open3D for point-cloud registration/basic ops specifically (narrower than Open3D's full pipeline, but much lighter).

**Open3D, resourced honestly**: CPU wheel is ~100-105MB — real, confirmed, and genuinely not "lightweight" by this machine's 7.6GB-RAM standard. Keep it named for Wave-3 scan-ingest (nothing else here replaces real point-cloud→mesh Poisson reconstruction); use #1-3 for everyday mesh work so the common case never pays Open3D's weight.

**Lightweight 3D object detection — no real case yet.** Real 2024-2025 research models exist (StripDet 0.65M params, PointeNet 1.4M params) but none are mature, pip-installable, off-the-shelf tools the way YOLO is. Flag as "revisit if a packaged release appears," not a current roadmap item.

All six new candidates fit `NEW_MCP_GUIDE.md`'s Pattern-A contract directly — none need a Rust modality-schema change (per `MODALITY_EXPANSION_GUIDE.md` §1's own boundary test).

---

## 3. The other 23 modalities — real inventory + 5 deep candidates

### Real coverage today
| Tier | Modalities | Real state |
|---|---|---|
| Has real external MCP | EEG, BCI (cerebrix-mcp); Image, Depth (visual-mcp); 3D, CAD (bridges) | Real data, proven |
| Partial | Biology, Network (connectome-mcp covers only the fly-neuron topology slice; biology's own scRNA/ATAC/pathway schema is untouched) | Rich real schema, narrow real data |
| **No real external tool, placeholder data** | Chemistry, DNA, Sonar, Kinematics, Hyperspectral, Thermal, Math, IMU, Proteomics, Haptic, Geospatial, Radar, Control, Electromagnetic, Sound | Real, correct domain *schemas* (confirmed genuinely well-designed, not stub naming) — but zero bundled real data, every graph writes `graphs/{modality}_placeholder.json` |

One resolved doc question: `sound/` vs `audio/` flagged as a possible duplicate in `MODALITY_EXPANSION_GUIDE.md` — **confirmed not a duplicate**. `audio` is general raw audio; `sound` is specifically bioacoustics (species call ID, echolocation analysis) with its own real cross-links to biology/geospatial/sonar.

### 5 deep, real, concrete candidates (chosen for tractability — a real free/no-auth API or a small bundleable real dataset, matching the cerebrix/connectome precedent exactly)
1. **DNA — Biopython + a bundled small real genome.** `Bio.SeqIO` reads FASTA/GenBank directly, pure-Python, near-exact structural clone of cerebrix-mcp's own bundled-trial-file shape. **High feasibility.**
2. **Chemistry — RDKit + PubChem PUG REST.** RDKit: ~29-35MB pip wheel, no GPU, no compiler. PubChem: free, no-auth, 100M+ real compounds by name/CID/SMILES. Fills chemistry's real `MoleculeAnalysis`/`Atom` types with genuine data. **High feasibility — closest match to connectome-mcp's own real-API+local-lib shape.**
3. **Proteomics — UniProt REST API** (`rest.uniprot.org`), free, no login, CC-BY-4.0. Pure HTTP+JSON, no ML, no install at all. **Very high feasibility — lightest option in this entire document.**
4. **IMU / Kinematics — UCI HAR dataset.** Public-domain, 30 real subjects, smartphone accelerometer+gyroscope, 6 real activity classes. Bundleable the same way KaraOne is. **High feasibility.**
5. **Geospatial — OpenStreetMap Overpass + Nominatim.** Free, no API key for reads, real worldwide data, generous limits (10k queries/day). `overpy` is a tiny pure-Python wrapper. **High feasibility — easiest of all five.**

**Honestly reported as having no good lightweight option found** (not stretched to fit): sonar (specialized XTF/JSF/S7K sensor-export formats, no casual free corpus), radar/electromagnetic/control (hardware-measurement domains, no free public API surfaced), hyperspectral (real benchmarks exist — Indian Pines/Salinas — but 100+MB with specialized `.mat`/ENVI readers, not lightweight by this doc's own bar), haptic and thermal (no real free public dataset found).

---

## 4. Cross-modal capability expansion

### What's already real today
1. **F.3 spatial relations on image detections** — real, live: YOLO bboxes → geometric `Above/Below/LeftOf/RightOf/NearTo/Supports`, persisted as real `ImageEdgeType` edges (confirmed live in container 40277).
2. **The depth-estimation bridge** — real, registered in the same model registry as YOLO (`kind` field distinguishes detection vs. depth in one shared registry — already a working cross-capability mechanism, not per-model bespoke code).
3. **The real generalized doctrine already exists**: `docs/LOCALIZATION_TRAVERSAL_GUIDE.md` already states "localize then traverse" as one reusable registry-of-models pattern, and already names CAD/3D/geospatial/chemistry/EEG as next candidates. **This is execution-ready, not undiscovered.**
4. **Confirmed honest gaps**: `InFrontOf` (depth-axis) is computed but never persisted as a real `ImageEdgeType` variant. Zero real link of any kind exists between EEG/connectome and anything visual (checked directly, confirmed absent).
5. **The real substrate**: `context.relationships` + `RelationType` is traversal-ready with zero traversal-side changes needed — `TraversalEngine::structural_traversal` already walks it like structural children. The existing cross-modal linker (3 independent copies across code/math/text `main.rs`) is purely lexical keyword overlap — no modality-type-aware linking exists at the host level yet.

### Real, concrete candidates (ranked by feasibility)
1. **Close the `InFrontOf` gap** — add the missing `ImageEdgeType` variant, wire `depth_graph`'s already-computed relation the same way F.3 relations already persist. No new model needed. **Trivial — hours, not days; the only reason this is open is deliberate deferral.**
2. **Depth + detection + spatial-relations fusion.** Real ultra-lightweight CPU depth alternatives exist beyond Depth-Anything-V2-Small: **FastDepth** (219K params, 10.49ms CPU), **MiniDepth** (415K params), **RT-MonoDepth-S** (667K params) — any could run alongside YOLO per-frame near-free. The fusion math (per-bbox depth sampling) is mostly already computed twice over by `depth_graph`; what's missing is purely the edge-persistence gap in #1 plus making `NearTo` depth-aware (distinguishing "overlapping but far in Z" from genuine proximity — a real correctness improvement, not just a feature). **High feasibility — mostly already built.**
3. **Image-region ↔ text grounding — Florence-2 or Grounding DINO.** Florence-2 (0.23B/0.77B, Microsoft, open-weight) does phrase grounding + dense region captioning directly. Grounding DINO does the inverse (text→region, zero-shot). Either registers like `yolo_detect` with `kind:"grounding"`. New relationship type needed: the already-declared-but-never-constructed `ImageEdgeType::DescribedBy`/`Describes` (confirmed schema-only today). **Moderate feasibility — real, but Florence-2-base is genuinely slower than YOLO; this is the real cost of the capability, not a framing problem to optimize around.**
4. **Image ↔ 3D "same real-world entity" link.** No model needed — a graph-architecture gap, not a capability gap. New relationship type: `RelationType::DepictsSameEntity` (or promote the already-declared, never-constructed `ImageEdgeType::Represents`/`ImplementedBy`), stored bidirectionally in `context.relationships`, matching the `ForkOf`/`ContinuedBy` idempotency-guard pattern already proven for AMT lineage. Honest start: a manual tag at creation time (zero-fabrication), not auto-matching via embeddings (a real, harder, separate future step). **High feasibility, no new model.**
5. **Sensor-array spatial fusion (EEG/BCI, radar, sonar)** — named as a structurally *different* problem, not image/3D's pattern: real referent is a fixed sensor topology, not a continuous scene. Right technique family: **source localization/beamforming** (dipole/source localization against known electrode positions) — a decades-old, well-documented, non-deep-learning technique, genuinely lightweight by construction. **Feasibility real but not concretely scoped** — flagged honestly as "right family known, specific build not sketched," since the real node schema is each modality's own (a §3 concern).

**Not recommended**: a general CLIP-style joint embedding space across all modalities. Real technique, but genuinely heavy (150M+ params even "small," real fine-tuning burden) and solves semantic-similarity search, not the actual gap here (geometric/positional relationships). The existing lexical `SimilarTo` linker already covers cheap semantic overlap.

---

## 5. Priority synthesis (across all 4 sections)

**Fix first (correctness, not expansion)**: §0 — the OCR/face-detection fabrication. This is a live integrity bug, ranked above every expansion candidate in this document.

**Cheapest, fastest, most honest wins** (zero new ML model, often zero new dependency):
- Classical OpenCV edge/line/shape detection (image, §1.2) — already installed.
- The `InFrontOf` edge-persistence fix (cross-modal, §4.1) — hours, not days.
- Proteomics via UniProt REST (§3.4) — pure HTTP, no install.
- Depth+detection+spatial fusion (cross-modal, §4.2) — mostly already computed.

**Best "cerebrix-mcp-shaped" new real MCPs** (bundled real data or a free real API, same proven shape):
- Chemistry (RDKit + PubChem).
- DNA (Biopython + bundled genome).
- Geospatial (OpenStreetMap).
- IMU/Kinematics (UCI HAR).

**Real but genuinely heavier — sequence deliberately, don't rush**:
- Segmentation via ultralytics' own `-seg` checkpoints (image).
- trimesh/numpy-stl/meshio/python-step-parser/cascadio (3D/CAD — six real additions to `MODALITY_EXPANSION_GUIDE.md`'s existing breadth).
- Florence-2/Grounding DINO region-text grounding (cross-modal) — real cost, not a framing problem.

**Honestly not pursued — no good lightweight option exists today**: image segmentation beyond ultralytics' own, lightweight 3D object detection, sonar/radar/electromagnetic/control/hyperspectral/haptic/thermal real data sources, general cross-modal CLIP-style embeddings.

Nothing in this document was built. Every candidate is named, real, and sourced — the next step is the operator's own prioritization call before any of this becomes code.
