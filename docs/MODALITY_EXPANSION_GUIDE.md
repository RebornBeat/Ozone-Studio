# Modality Expansion Roadmap — the MCP/tool family across all live modalities, localization, and the physical-bridge vision

> Tenth doctrine doc; the comprehensive capture CC was writing when its
> session hit the limit. Covers: the tools-expand-graphs contract (the
> learning point), the localization-model families across all live
> modalities (3D/CAD/IMAGE/AUDIO/VIDEO), the environment-scanning →
> Blender → CAD → 3D-printing physical bridge, the full pairwise
> cross-modality combination matrix (§4 — 10 core pairs + 5 CODE-bonus
> pairs, graded live/real/narrow/none — never padded), and the full
> MCP/tool roadmap (§5 — 44 numbered + 2 sub-items = 46 real named
> candidates as of 2026-09-29, one master table plus wave sequencing,
> combinatorial toward 100+). Companion to:
> NEW_MCP_GUIDE (Patterns A-C), REVERSE_ENGINEERING_GUIDE, TOOLS_REGISTRY,
> TOOLS_PIPELINES_MCP_GUIDE (§11.6 detection, §11.3 edge-identification).

---

## 1. THE CONTRACT (the learning point, formalized)

**Tools expand the graph. Modalities define the graph.**

A modality pipeline (text 100, code 101, image 102, audio 103, video 104,
math 105, 3D 109, CAD 120) OWNS its node types, edge types, and
persistence contract — defined in Rust, changed by a build. Tools are
REGISTERED capabilities (JSON registry entries + MCP surfaces) that RUN
models/pipelines and EXPAND the graph through the standard choke point —
no modality source changes, no builds, add-one-entry expandability.

**The boundary test** (applied before any new capability):
- Does it create/modify node types or edge types in the Rust modality
  source? → That's a MODALITY change (build + restart required).
- Does it run a model/pipeline and write containers/edges through the
  existing choke point? → That's a TOOL change (registry entry, live
  immediately).

The depth work (2026-09-29) is the worked example for the TOOL side:
depth_estimate and depth_graph landed as TOOLS (same-day, registry-driven,
no host rebuild) — but the `InFrontOf` typed edge they compute is returned
in the response, NOT persisted as an edge, because image 102's
`ImageEdgeType` enum has no depth-axis variant. That missing variant is
the one legitimate modality change queued (a real Rust schema addition,
deliberately its own pass).

The call-graph work (2026-09-29, same day) is the mirror-image worked
example for the MODALITY side: entry-point detection and call ordering
landed as `compute_call_ordering()` INSIDE pipeline 101's own Rust, not
as a tool — because Calls edges + ordering ARE what a call graph means,
deterministic structural analysis, not an external model bolted on. The
`code_callgraph` MCP tool that wraps it is still the expandable surface
(exactly like `yolo_detect` wraps YOLO) — but the ANALYSIS lives in the
modality, not the tool. Applying the boundary test from two real,
opposite, same-day examples: depth estimation is a swappable external ML
model (tool), call ordering is intrinsic graph structure (modality) —
together they make the distinction concrete rather than abstract for the
next capability anyone has to classify.

**Corollary — the expansion contract for future tool families:**
1. A tool NEVER invents new edge/node types; it uses the modality's
   existing types, or requests a modality schema addition (Rust, built,
   versioned) — additions only, never rewrites.
2. A tool's detections/measurements become node ATTRIBUTES freely
   (attributes are JSON — depth_median landed this way with zero schema
   change); typed EDGES require the modality-schema path.
3. N tools of the same family run in parallel on the same input and merge
   through the choke point (YOLO + Depth-Anything proved this: two model
   families, one registry, one graph).

---

## 2. LOCALIZATION-MODEL FAMILIES per live modality

The depth work established the pattern: one registry
(`detection_models/registry.json`, kind-tagged), one tool family per
model kind, each expanding its modality graph. The families across all
live modalities:

### CODE (101) — LIVE (added here — was missing from this table despite
being built the same day; see §1's worked example)
| Kind | Source | Expands | Status |
|---|---|---|---|
| call-graph | `compute_call_ordering()` — native Rust, pipeline 101 | `call_order`/`is_entry_point` Function-node properties; `entry_points`/`call_sequence` on the analysis result | ✓ LIVE — container 40425 |
| MCP surface | `tools/code-mcp/server.mjs`, `code_callgraph` tool | wraps the above through `/mcp/call`, optional persist | ✓ LIVE |
| static-analysis | (queued) real linter/complexity-metric integration (ruff/clippy/eslint-family — language-dependent, no single tool covers all) | diagnostic node attributes | registry entry when built |

### IMAGE (102) — LIVE
| Kind | Model (registered) | Expands | Status |
|---|---|---|---|
| detection | yolov8n (ultralytics, coco80) | Object nodes + F.3 spatial edges | ✓ LIVE |
| depth | Depth-Anything-V2-Small (transformers) | per-object depth attributes + InFrontOf relations | ✓ LIVE |
| segmentation | (queued) SAM/SegAny family | mask attributes, precise boundaries | registry entry when weights land |
| pose | (queued) YOLO-pose / MediaPipe | keypoint nodes, skeleton edges | registry entry |

### 3D (109) — LIVE pipeline + Blender bridge
| Kind | Model/Source | Expands | Status |
|---|---|---|---|
| scene-import | Blender MCP bridge (LIVE: real scene → 5-node graph, container 40232) | Object/Scene nodes, transforms | ✓ LIVE |
| photogrammetry | RealityCapture/Meshroom output → mesh import | Mesh nodes with real measurements | via Blender bridge |
| LIDAR/point-cloud | scanner export → point-cloud → mesh → graph | point nodes → merged mesh nodes | **the operator's scan vision — see §3** |
| SLAM | phone AR (ARCore/ARKit) session export | camera-pose nodes + spatial graph | phone-bridge extension |

### VIDEO (104) — pipeline compiled, persistence wired
| Kind | Source | Expands | Status |
|---|---|---|---|
| frame-locals | per-N-frame image 102 Analyze+YOLO | temporal Object nodes + TemporalPrecedes edges | recipe proven (image family), wiring queued |
| scene-cut | shot-boundary detection | Scene nodes, NarrativeLeadsTo edges | same |

### AUDIO (103) — pipeline compiled + persistence
| Kind | Source | Expands | Status |
|---|---|---|---|
| source-localization | stereo/beamformer direction estimate | directional origin attributes + spatial edges | model research queued (CC flagged: needs real model work per modality, not rushed) |
| transcription | Whisper (voice pipeline — NOT yet built) | text nodes → cross-linked to audio nodes | voice build queued |
| event-detect | onsets/spectral events | event nodes + TemporalPrecedes chains | queued |

### CAD (120) — the physical-bridge terminus
| Kind | Source | Expands | Status |
|---|---|---|---|
| parametric-import | FreeCAD/Step/DXF parse → parametric graph | ParametricNode with real dimensions | pipeline compiled (CAD compiled per CC) |
| scan-to-CAD | scan mesh → CAD reference (§3 terminus) | measured references with real units | the operator's end goal |

---

## 3. THE PHYSICAL BRIDGE: scan → Blender → CAD → print

The operator's environment-scanning vision, as a staged pipeline:

```
[Physical environment]
   │  scan: phone AR session / LIDAR / photogrammetry (photo set)
   ▼
[Point cloud / mesh]  ── real measurements survive every hop
   │  import: Blender MCP (bridge 203) → ThreeD graph nodes
   ▼
[3D modality graph (109)]  ── spatial relations, transforms, versions
   │  export: graph → Blender scene (bridge round-trip) → CAD reference
   ▼
[CAD (120) parametric graph]  ── measured, editable, real units
   │  export: STEP/DXF → slicer
   ▼
[3D print / fabrication]
```

**What exists TODAY (live)**: Blender bridge (real scene reads — proven
against the operator's own scene, Ozone_Cube_01), 3D graph persistence
(container 40232), image depth (InFrontOf bootstrap), terminal for
slicer/tool invocation.

**What's queued (in order)**:
1. **Scan ingest MCP** (`scan-ingest`): photogrammetry outputs (COLMAP/
   Meshroom/Metashape export) → point cloud/mesh files → Blender import
   via bridge 203 → ThreeD graph. The LIDAR/SLAM sensor choice is
   downstream of this — ANY source producing a mesh lands here the same.
2. **Measurement preservation**: scan scale reference (a real object of
   known length in frame) → real-units attribute on the graph; wood/
   plywood/objects get measured dimensions as node attributes.
3. **Blender round-trip**: graph edits → bridge 203 execute calls → real
   Blender scene mutations (proven path: blender_get_scene/create_object
   already live).
4. **CAD hand-off**: mesh → parametric reference conversion (FreeCAD
   headless or STEP export via Blender) → CAD 120 modality graph.
5. **Print hand-off**: CAD → STL/3MF export → slicer invocation via
   terminal MCP (allowlisted: the slicer binary only).

Every arrow is a standard contract hop: register → heartbeat → execute
under /mcp/call, metered/gated/ripped/reviewed/captured.

---

## 4. CROSS-MODALITY COMBINATION OPERATIONS (the operator's ask: deconstruct /
reconstruct / merge / localization / capture, across 3D/CAD/IMAGE/AUDIO/VIDEO)

The operator named five operation KINDS explicitly and asked for the full
rundown of how tools combine across 3D/CAD/IMAGE/AUDIO/VIDEO — all 10
pairs, not a sample. These aren't new architecture — they're names for
moves the system already makes, worth making explicit so the next tool
is classified correctly on sight. §4.1 covers the within-one-modality
case (already proven); §4.2 is the complete 10-pair cross-modality
matrix the operator asked for; §4.3 adds CODE as an honest bonus sixth
modality, since two of its five pairings are already true today.

- **Localization** — find/place discrete things inside one modality's
  space. Always FIRST; nothing downstream has anything to traverse until
  something is localized. Real examples: YOLO boxes (image), entry-point +
  call-order (code), SAM/pose keypoints (image, queued), feature/tolerance
  extraction (CAD).
- **Capture** — get raw modality data INTO the graph from something
  external to it. A capture tool wraps a device/API and feeds the
  modality's own `Analyze` action; it never invents graph types itself.
  Real/queued examples: screenshot (image, via browser or OS), scan/
  photogrammetry (3D), audio recording, frame extraction (video), a design
  file parse (CAD).
- **Deconstruct** — break one captured/localized thing into its
  constituent typed nodes/edges WITHIN one modality graph. This is what
  `Analyze` already does everywhere; naming it as its own operation kind
  makes "is this tool a deconstruct step" a fast classification question.
  Real examples: image → per-object nodes + spatial edges (live);
  call-graph → Function nodes + Calls edges + call_order (live); CAD →
  assembly → parts → features (pipeline compiled, not yet registered).
- **Reconstruct** — the inverse: take graph data and regenerate a concrete
  artifact. This is what makes the §3 physical-bridge round-trip real
  rather than one-directional. Real/queued examples: CAD parametric graph
  → STEP export (queued, §3 step 4); 3D graph → Blender scene mutation
  (live path, blender_get_scene/create_object); image graph → annotated
  overlay (trivial, not yet built as its own tool).
- **Merge** — combine two or more graphs (or two tool outputs within one
  modality) into one enriched graph through the SAME choke point the
  system already uses for any container write. Merge is where "how do
  tools combine" actually lives.

**§4.1 — Same-modality merge (the rule already proven).** N tools in one
modality run on the same input and combine through the existing choke
point — no cross-modality complexity, just parallel tool outputs on one
container. ✓ LIVE proof: YOLO detection + Depth-Anything depth → one
image graph (`depth_graph` tool, depth attrs land on the Object nodes
YOLO already created). Same pattern applies the moment any second tool
in a family ships (e.g. segmentation + detection + pose, all on one
image, all on one graph) — not a new mechanism each time, the same one.

**§4.2 — THE FULL CROSS-MODALITY PAIR MATRIX.** All 10 unique pairs
across the operator's five named modalities (3D/CAD/IMAGE/AUDIO/VIDEO),
graded honestly — a pairing gets a real mechanism when one exists, and
an explicit "no strong case" when it doesn't rather than an invented one:

| # | Pair | Status | Mechanism |
|---|---|---|---|
| 1 | IMAGE × AUDIO | queued, narrow | Face detection (image) + diarized speaker segment (audio) → lip-sync/speaker-ID match. Needs synchronized streams, not a lone photo + lone clip — the strong version of this pairing runs through VIDEO (#2/#5 below), not standalone. |
| 2 | IMAGE × VIDEO | recipe proven, wiring queued | This pairing IS the `frame-locals` mechanism already named in §2: video decomposed into per-N-frame image `Analyze`+YOLO calls. Not a separate build — the existing VIDEO row. |
| 3 | IMAGE × 3D | queued, real path exists | A detected object (photo) matched to a scanned mesh object representing the same physical thing. Blender bridge already imports both graphs; matching is a new small tool (spatial/visual proximity), not new architecture. |
| 4 | IMAGE × CAD | queued, real, buildable | Photo vs CAD-rendered comparison — part ID, QC, tolerance/defect check. Mechanism: CLIP embedding similarity (§5 item 9) or feature match between a real photo and a CAD-generated render. Needs a compare tool, not new architecture. |
| 5 | AUDIO × VIDEO | queued | Audio event timeline ↔ video scene-cut timeline merge into one AV timeline — both are already `TemporalPrecedes` chains, merge = shared timestamp axis, no new edge type needed. |
| 6 | AUDIO × 3D | queued, real mechanism | Source-localization direction estimate (§2 AUDIO row) → a directional edge/attribute from a sound-origin point to the nearest 3D graph object ("the noise is coming from behind the cabinet"). Combines two already-named capabilities, no new tool family. |
| 7 | AUDIO × CAD | **no strong case identified** | The only plausible use — acoustic/ultrasonic non-contact measurement feeding CAD dimensions — requires measurement hardware we have no evidence of owning. Flagging honestly rather than inventing a use case to fill the cell. |
| 8 | VIDEO × 3D | queued, real mechanism | Multi-frame photogrammetry FROM video (moving-camera structure-from-motion) reconstructs a dynamic/4D scene. **COLMAP (§5 item 20) already accepts video-frame input directly** — this is a config choice on an already-named tool, not a new one. |
| 9 | VIDEO × CAD | queued, real, names an existing CAD capability | Assembly/build verification: video of a physical build compared against the CAD assembly's BOM/sequence. `CADGraphQuery::GenerateBOM` is a real, already-written enum variant (confirmed during the CAD compile-fix investigation) — this pairing is a genuine 3-way blend with IMAGE (per-frame detection) under the hood, not a 2-tool merge. |
| 10 | 3D × CAD | queued, well covered | The physical-bridge terminus itself — scan-to-CAD, §3's entire staged pipeline. Already the most-detailed pairing in this doc. |

**§4.3 — CODE as a sixth modality (bonus — the operator's four named
modalities plus the one already proven live twice today).** CODE wasn't
in the operator's named list, but two of these five pairings are already
how the live system works today, worth naming explicitly rather than
leaving implicit:

| # | Pair | Status | Mechanism |
|---|---|---|---|
| 11 | CODE × 3D | **effectively ✓ LIVE already** | Blender is Python-scriptable — the live bridge's `create_object`/`get_scene` calls ARE code driving the 3D modality under the hood. Not a new build; naming what's already true. |
| 12 | CODE × CAD | queued, real, well-aligned | FreeCAD is Python-scriptable (a real, standard FreeCAD capability) — macro/script-driven parametric CAD generation feeds directly into the Wave-3 CAD hand-off step (§5 item 23), using the same FreeCAD tool already named there. |
| 13 | CODE × IMAGE | real but narrow, not prioritized | Script-driven synthetic image generation for test fixtures/training data. |
| 14 | CODE × AUDIO | real but narrow, not prioritized | Procedural/programmatic audio generation. The real near-term audio priority is the voice/Whisper-or-replacement path (§2 AUDIO row), not this. |
| 15 | CODE × VIDEO | real but narrow, not prioritized | Scripted video editing/generation (e.g. Blender's video sequence editor via Python). |

The pattern holds across every pairing in both tables: **merge never
invents a type**, it either adds attributes to existing nodes (cheap,
live today) or requires one modality-schema addition when a genuinely
new relation crosses domains (the `InFrontOf`/ImageEdgeType gap in §1 is
the one real example of the latter so far). Where no honest mechanism
exists (#7), the matrix says so instead of padding it.

---

## 5. THE MCP/TOOL ROADMAP — honest count as of 2026-09-29

Below are the real, individually named tool/model candidates identified
so far. **Honesty note, corrected in this pass**: an earlier version of
this doc labeled Wave 2 "17 items" and claimed a 46-item total — both
wrong by recount (Wave 2 only ever listed 15 items, 5-19). The real,
checked total is **44 distinct named candidates** (existing open tools/
models, not fabricated) — not a padded 100. The 20-100 scale the operator
described is real but COMBINATORIAL, not a flat list: each Wave-2 model
family × each modality it applies to × each §4 merge pairing multiplies
out well past 100 concrete registry entries over time (e.g. one
segmentation model registered once, applied to image/video/CAD-drawing
alike). This section tracks the distinct TOOLS; §4 tracks how they COMBINE.

### §5.1 — MASTER LIST (all 44, one table, the full rundown)

| # | Tool / Model | Modality | Operation (§4) | Status |
|---|---|---|---|---|
| 1 | web-search MCP | cross-cutting | capture | ✓ LIVE (current_datetime proven; web_search awaits Brave key) |
| 2 | browser MCP | cross-cutting | capture | not being built now (operator, 2026-09-29) |
| 3 | git MCP | cross-cutting | capture | not being built now (operator, 2026-09-29) |
| 4 | S13 read route (`GET /capture/tool-calls`) | cross-cutting | capture | queued |
| 5 | segmentation — SAM2 / MobileSAM | image 102 | deconstruct | candidate |
| 6 | pose — YOLO-pose / MediaPipe Pose | image 102 | localization | candidate |
| 7 | OCR — Tesseract / PaddleOCR / TrOCR | image 102 | deconstruct | candidate |
| 8 | face/landmark — MediaPipe FaceMesh | image 102 | localization | candidate — **privacy/consent decision needed before build** |
| 9 | zero-shot tagging — CLIP | image 102 | deconstruct | candidate — also the IMAGE×CAD (§4.2 #4) compare mechanism |
| 10 | super-resolution/inpainting — Real-ESRGAN / LaMa | image 102 | reconstruct | candidate |
| 11 | optical flow — RAFT | image 102 | localization | candidate — feeds video frame-locals |
| 12 | speaker diarization — pyannote.audio | audio 103 | localization | candidate |
| 13 | source separation — Demucs / Spleeter | audio 103 | deconstruct | candidate |
| 14 | non-speech event classification — PANNs / YAMNet | audio 103 | localization | candidate |
| 15 | beat/tempo tracking — librosa | audio 103 | localization | candidate |
| 16 | frame interpolation — RIFE | video 104 | reconstruct | candidate |
| 17 | cross-frame object tracking — ByteTrack / DeepSORT | video 104 | merge (same-modality) | candidate |
| 18 | action recognition — VideoMAE-family | video 104 | deconstruct | candidate |
| 19 | metric-depth models (real units) | image 102 / 3D 109 shared | localization | candidate — needed before §3 step 2 |
| 20 | scan-ingest — COLMAP (photo-set/video SfM) | 3D 109 | capture | candidate — also the VIDEO×3D (§4.2 #8) mechanism |
| 20a | scan-ingest — Open3D (point-cloud → mesh) | 3D 109 | capture/deconstruct | candidate |
| 20b | scan-ingest — ORB-SLAM3 (live SLAM session) | 3D 109 | capture | candidate |
| 21 | measurement preservation (scale reference → real-units attrs) | 3D 109 | deconstruct | queued (§3 step 2) |
| 22 | Blender round-trip graph-edit path | 3D 109 | reconstruct | ✓ LIVE path (blender_get_scene/create_object) |
| 23 | CAD hand-off — FreeCAD headless | CAD 120 | reconstruct | candidate — also the CODE×CAD (§4.3 #12) mechanism |
| 24 | print hand-off (slicer invocation, allowlisted) | CAD 120 | reconstruct | queued (§3 step 5) |
| 25 | chemistry 106 revival | chemistry 106 | — | compiled, unrevived |
| 26 | dna 107 revival | dna 107 | — | compiled, unrevived |
| 27 | eeg 108 revival | eeg 108 | — | compiled, unrevived |
| 28 | BCI revival | BCI | — | real dir, unrevived |
| 29 | biology revival | biology | — | real dir, unrevived |
| 30 | control revival | control | — | real dir, unrevived |
| 31 | depth (modality dir) revival | depth | — | real dir, unrevived — **name collision with the image-102 depth model (item 19/§2), unresolved** |
| 32 | electromagnetic revival | electromagnetic | — | real dir, unrevived |
| 33 | geospatial revival | geospatial | — | real dir, unrevived |
| 34 | haptic revival | haptic | — | real dir, unrevived |
| 35 | hyperspectral revival | hyperspectral | — | real dir, unrevived |
| 36 | IMU revival | IMU | — | real dir, unrevived |
| 37 | kinematics revival | kinematics | — | real dir, unrevived |
| 38 | network revival | network | — | real dir, unrevived |
| 39 | proteomics revival | proteomics | — | real dir, unrevived |
| 40 | radar revival | radar | — | real dir, unrevived |
| 41 | sonar revival | sonar | — | real dir, unrevived |
| 42 | sound revival | sound | — | real dir, unrevived — **unresolved: distinct from `audio/` or legacy/redundant, nobody's checked its real content** |
| 43 | thermal revival | thermal | — | real dir, unrevived |
| 44 | CAD 120 layer-1 persistence + index registration | CAD 120 | — | queued (compile errors already fixed) |

(Items 20a/20b are the three concrete tools behind Wave-3 item 20's
"scan-ingest" umbrella, named individually here since the master list's
whole point is not compressing distinct real tools into one line — they
don't get their own top-level numbers since they're one MCP surface
wrapping three interchangeable backends, same registry pattern as
detection/depth today.)

**True flat count: 44 numbered items + 2 sub-items (20a/20b) = 46 real
named things** if sub-items are counted individually — which resolves
the apparent contradiction between this correction and the number stated
in earlier handoff notes: those notes said 46 counting COLMAP/Open3D/
ORB-SLAM3 as three separate things (correct, they are), while the Wave
heading arithmetic undercounted by collapsing them into one Wave-3 item.
Both framings are now consistent in this version.

### Wave 1 — wraps BUILT pipelines (fastest, ~1 day each) — 4 items
1. web-search MCP ✓ (current_datetime PROVEN; web_search awaits Brave key)
2. browser MCP — wraps browser_navigation 25: navigate/read/screenshot;
   screenshot chains into visual → YOLO → depth → graph (RE visual loop).
   **Not being built right now per operator instruction (2026-09-29) —
   listed for completeness only.**
3. git MCP — typed git operations (status/diff/log), FileChange evidence.
   **Not being built right now per operator instruction (2026-09-29) —
   listed for completeness only.**
4. S13 read route — GET /capture/tool-calls (CC's UI family wants it)

### Wave 2 — model families (registry-driven, data-first) — 15 items (corrected; was mislabeled 17)
Real, named, open models/tools per modality (named the same way
Depth-Anything/MiDaS/COLMAP were — real and commonly used, not yet
vetted by us for this codebase):

**IMAGE (102) — beyond YOLO+depth, already live:**
5. segmentation — SAM2 or MobileSAM (lighter) → mask attributes
6. pose — YOLO-pose (same family as the registered yolov8n) or
   MediaPipe Pose → keypoint nodes + skeleton edges
7. OCR — Tesseract (classic) or PaddleOCR/TrOCR (HF) → text-in-image,
   cross-links to text-modality (100) nodes
8. face/landmark — MediaPipe FaceMesh. **Flagged explicitly: this needs
   a privacy/consent policy decision before building, not just a tech
   spec — face data is sensitive in a way boxes/depth aren't.**
9. zero-shot scene/category tagging — CLIP (OpenAI, HF-hosted) → tag
   attributes without a fixed class list (unlike YOLO's coco80)
10. super-resolution / inpainting — Real-ESRGAN / LaMa → reconstruct-
    family, upscale or fill before (re-)analysis
11. optical flow — RAFT → motion vectors between a frame pair, feeds
    video frame-locals (item 14) as a real per-pair signal

**AUDIO (103) — beyond the three already named in §2:**
12. speaker diarization — pyannote.audio (HF-hosted) → WHO spoke WHEN,
    distinct from Whisper's WHAT
13. source separation — Demucs or Spleeter → deconstruct a mixed track
    into stems, each stem re-enters the graph as its own audio sub-node
14. non-speech event classification — PANNs or YAMNet → event tags
    (glass break, dog bark) distinct from onset-only event-detect
15. beat/tempo tracking — librosa-based (classic DSP, not ML) →
    TemporalPrecedes anchors for music content

**VIDEO (104) — beyond frame-locals/scene-cut already named in §2:**
16. frame interpolation — RIFE → reconstruct-family, fill temporal gaps
17. cross-frame object tracking — ByteTrack or DeepSORT → the merge
    listed in §4 (one persistent Object node + track_id, replacing
    independent per-frame re-detection)
18. action recognition — VideoMAE-family (HF-hosted) → clip-level
    action tags as Scene node attributes

**Metric depth (image/3D shared frontier):**
19. metric-depth models (real units, not disparity-space) — the
    named next step beyond Depth-Anything's relative depth, needed
    before depth attributes can feed real-unit CAD measurements (§3
    step 2)

### Wave 3 — the physical bridge (§3), tools now named concretely — 5 items
20. scan-ingest MCP — concrete tools behind "photogrammetry/LIDAR mesh
    import": **COLMAP** (the standard open structure-from-motion tool)
    for photo-set input, **Open3D** (point-cloud → Poisson mesh
    reconstruction) for point-cloud input, **ORB-SLAM3** (the standard
    open visual-SLAM implementation) for a live phone/camera session
21. measurement preservation (scale reference → real-units attributes)
22. Blender round-trip graph-edit path (live path already proven)
23. CAD hand-off — **FreeCAD** headless (already named in §2's CAD row)
    for parametric reference conversion / STEP export
24. print hand-off (slicer invocation via terminal MCP, allowlisted)

### Wave 4 — the remaining modality revivals (mechanical recipe, ~30min each by script) — 20 items
25-27. chemistry 106, dna 107, eeg 108 (already named elsewhere, compiled).

28-43. The remaining 16, named explicitly here (real directory inventory,
checked directly against `assets/pipelines/modalities/` during the CAD
investigation — not estimated): **BCI, biology, control, depth (a
distinct modality directory — not to be confused with the image-102
depth *model* in §2, worth someone resolving the naming collision),
electromagnetic, geospatial, haptic, hyperspectral, IMU, kinematics,
network, proteomics, radar, sonar, sound, thermal**.

One real finding worth flagging here, not resolved: **both `audio/` and
`sound/` exist as separate modality directories** — unclear whether these
are meant to be distinct capabilities or one is legacy/redundant; nobody
has looked at `sound/`'s real content yet to know which.

44. CAD 120 — layer 1 (persistence + `zsei_data/pipelines/index.json`
registration) still open; compile errors already fixed (CC, this session).

Every wave composes: localization feeds deconstruction feeds merge feeds
reconstruction feeds capture-of-the-next-thing — one registry, one choke
point, one order (§4.2/§4.3's matrices are the concrete "how they combine"
the operator asked for; §5.1's master table is the concrete "full list").

**Scope note (operator, 2026-09-29): git MCP and browser MCP (Wave 1,
items 2-3) are explicitly NOT being built right now** — this pass is
guide/documentation expansion only. Nothing in Wave 2-4 has been started
either; all of §5 is candidate inventory, not a build log.

---

## 6. REGISTRY/VERSIONING NOTES (current state, honest)

- detection_models/registry.json v0.2.0: kind-tagged (detection/depth),
  3 entries, per-model platform+modality tags.
- /mcp/tools: 75+ tools, persisted (mcp_tool_registry.json), restored
  across restarts (verified 3x). Version REPLACE works; range-dispatch
  and native platform field designed-not-built.
- Roles: per-agent allowlists (roles.json next to terminal server),
  proven 3-way; identity validation live on /orchestrate + /mcp/call.
- Auth half-2 (queued): Electron UI attaching Ed25519 sessions to
  /orchestrate and /mcp/call — makes roles key on VALIDATED identity.
