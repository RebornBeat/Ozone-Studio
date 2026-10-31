# Geometry guide — every modality, current state, crosswalk, target (2026-10-31)

Operator direction: capture insights on ALL modalities and the current
geometrical state of each, the crosswalks, and the new findings; geometry is
intrinsic, traversable and spans graphs; depth in images is not full geometry
the way other modalities have it; remember how modalities work with each other
(cross-modality tool calls, per-modality runs, MCPs). Review and capture; the
state below was READ from the code by four audits (A1-A4, file:line cited in the
audit reports) and spot-verified. Companion design: `SPATIAL_GEOMETRY_LAYER.md`.
Nothing here was built or run.

## 0. Headline findings (new)

1. **The image pipeline does not look at pixels.** `analyze_image`
   (`assets/pipelines/modalities/image/main.rs:1234`) loads the bytes, discards
   them (`_image_bytes`), and returns constants: object `example_object` at
   (100,100,200,150), 800x600 PNG, sRGB, 8-bit, fixed colours and composition
   (comment: "In production, this would call actual CV models"). I verified this.
   **Consequence: the EU `/capture/ingest_image` "real image analysis (bit depth,
   sRGB, composition)" reported earlier was these constants, not analysis of
   the farm screenshot.** The ingest route, gate and dispatch are real; the
   analysis content is not. The only pixel-derived data in the system comes
   from visual-mcp (YOLO boxes/masks/pose, cv2 lines and contours,
   Depth-Anything relative depth).
2. **Most "analysis" modalities are typed shells.** Video returns a canned
   120 s / 30 fps / "person" track. 3D `AnalyzeScene` labels the file format and
   returns defaults (fps 24). CAD `AnalyzeFile` labels the format; STEP/IGES are
   not parsed. Depth never parses PCD/PLY/LAS. EEG has hard-coded channels.
   Proteomics has no `ATOM` parser. Chemistry returns a hard-coded 2-atom
   "MMFF94" structure. Radar/EM/sonar carry "In production: FFT/CFAR/DoA"
   comments with no algorithm. Geometry TYPES are rich; geometry VALUES are mostly
   absent.
3. **Spatial relations are mostly not computed from geometry.** They are
   mapped from strings produced by LLM zero-shot (3D `ComputeSpatialRelationships`
   ignores its `SpatialMethod`; geospatial `infer_geographic_relationships`;
   haptic/thermal/hyperspectral edge names). Real geometric computation exists
   in: image `CreateGraph` (IoU Overlaps + centre-based Above/Below/LeftOf/
   RightOf), geospatial (haversine, point-in-polygon), kinematics (forward
   kinematics), CAD (AABB interference), mesh-mcp (trimesh numbers).
4. **Four edge vocabularies, no common set.** Host `RelationType`
   (`src/types/container.rs:295-301`) has only Above/Below/NearTo/InFrontOf/
   Overlaps. `mcp_graph.relation_from_name` (`src/mcp_graph.rs:97-110`) accepts
   only those plus structural names, and **rejects the whole graph block on an
   unknown relation** (`:389-390`), so LeftOf/RightOf/Supports/Touches/Behind
   cannot be sent. Image's own enum lacks InFrontOf/NearTo; 3D and depth have
   richer private enums.
5. **No geometry field in the graph.** `mcp_graph` nodes carry only `key, kind,
   label, parent, attributes`; geometry rides in free-form attributes behind an
   object-store path. `modality_for` labels everything except `visual_/yolo_/
   depth_/shape_/pose_` tools as `Unknown` (dna, chemistry, geospatial, mesh,
   browser, android graph blocks all `Unknown`).
6. **No shared frame concept.** Each pipeline has its own transform or tuple
   convention. Depth has a `CoordinateSystem` enum (SensorLocal/ENU/NED/ECEF/
   UTM/WGS84) and `Transform4x4`; 3D has `Transform4x4`; haptic has a 4x4 stored
   but unused; IMU has a mounting frame; geospatial has a CRS. None is linked.
7. **Cross-modality links do not exist as geometry.** `DepictsSameEntity`,
   `Renders`, `Locates` appear nowhere. Sensor "cross-modal" edges are
   self-loops with target ids in properties (thermal even says "Remove
   cross-modal self-loops that served as placeholders"). The only real linker
   is model-driven (`src/zsei/hooks.rs:379-710`), choosing among
   Describes/Implements/Represents/Documents/Illustrates.
8. **Depth is not full geometry.** What exists is Depth-Anything-V2-Small
   *relative* per-box depth from visual-mcp (median/mean per detection;
   `InFrontOf` return-only, never persisted). The depth modality has the best
   geometry TYPES (intrinsics, `BoundingBox3D`, planes, meshes, coordinate
   systems) but reads no point cloud and does no back-projection.
9. **Ambiguity:** URDF/MJCF/SDF extensions appear in both the 3D and
   Kinematics extension tables (`src/types/modality.rs:157-160` vs `:196`), so
   detection of those files is ambiguous.
10. **Binary state:** only audio, code, image, math and text have a built
    binary in their own `target/release`; the host may resolve others via a
    builtin path, so confirm against the registry before concluding.

## 1. Per-modality geometry state (what is REAL vs typed)

Legend: **R** real computation/values present · **T** types declared, values
absent/placeholder · **L** LLM-inferred · **–** none/not applicable.

| Modality (id) | Primitives & frame | Values | Spatial relations | Notes |
|---|---|---|---|---|
| Image (102) | BoundingBox(+rot), RLE mask, polygon region, LeadingLine, DepthLayer; frame image px (implicit) | pipeline **T/const**; visual-mcp **R** (YOLO boxes/masks/pose, Hough lines, contours, relative depth) | pipeline persists Overlaps/Above/Below/LeftOf/RightOf (**R**, centre-based); NearTo/Supports/InFrontOf return-only | no pixel decode in pipeline; no scale/frame model; shape+seg graphs have no edges |
| Video | BoundingBox, ObjectTrack, TimestampedBox | **T/const** (canned) | temporal only | no frame extraction, no motion geometry |
| Depth (115) | Transform4x4, CameraIntrinsics, BoundingBox3D, planes, meshes, CoordinateSystem enum | **T** (no readers; placeholder RMSE) | Occludes/DistanceTo/SupportedBy/AbovePlane (**T**) | image-depth bridge is relative only (see 0.8) |
| 3D (109) | Transform4x4, BoundingBox3D, scene graph, mesh arrays; frame world, up_axis, units | **T** (no scene readers); bridge/Blender tools give **R** transforms but pipeline does not call them | richest vocab (Above/Behind/Intersects/Supports…) but **L** | BuildSpatialIndex stub |
| CAD (120) | CADTransform, CADBoundingBox, sketch entities, FaceInfo/EdgeInfo/VertexInfo, constraints | **T** (STEP/IGES not parsed); AABB interference **R** | B-rep topology edges (**T**), no spatial | DXF/SVG export-only; mesh-mcp `cad_step_info` structure only |
| Kinematics (121) | DHParameters, Pose6DOF, LinkBoundingBox, WorkspaceVolume | forward kinematics **R** | chain topology only | URDF/MJCF import/export stubs |
| Geospatial (117) | GeoPoint/LineString/Polygon(holes)/BBox, CRS transform, vertical datum | haversine, point-in-polygon **R** | LocatedIn/ContainsPOI **R**; others **L** | spatial index declared, not implemented; best CRS handling |
| Chemistry | Position3D, Structure3D, Atom/Bond | **fabricated** 2-atom structure | Contains/BondedTo only | MCP graph block has no coordinates |
| Proteomics (112) | residue numbers, SS intervals, pLDDT | **T** (no x/y/z, no ATOM parser) | interaction/interval edges | |
| DNA | 1-based sequence intervals | **R** (GenBank via MCP) | contains | no 3D/chromatin |
| Biology (111), Math, Control (122) | – (keyword counts only) | – | – | not individually inspected |
| Network (123) | optional `geo_location` | metadata | topology edges | header claims geo linking (117); no edge code |
| Text (100) | char span `TextPosition` (offset,line,col) | **R** spans | none | PDF/DOCX/RTF listed but **never read**: no page/line boxes, no reading order |
| Code (101) | line/col spans (columns mostly None) | **R** lines | none (1-D layout) | embedded HTML/CSS/script link to UiDocument not modeled |
| Audio | time spans, channel count | **R** time | `Overlaps` is temporal | no spectrogram grid, no stereo direction |
| Sound | mic/element positions, SourceLocalization record, LocalizationMethod enum | **T** (localization never filled; node loop dead) | `LocalizedAt` mapped, not derived | PlottedOnGeoMap self-loop |
| EEG | electrode `Position3D` | **T** (stub, hard-coded channels) | none | no montage/head frame |
| BCI | ElectrodeGeometry positions_um | **T** (ignored by sorter) | none | no EEG link |
| IMU | quaternion, mounting frame, 3-vectors | **R** fusion types; position dead-reckoned | none | frame types not registered to a parent |
| Haptic | pressure grid, contact point/normal, 4x4 sensor-to-world | **T** (4x4 stored, unapplied) | names mapped only | |
| Thermal | pixel grid, ROI, geo_location | **T** | names mapped only | no registration to image |
| Electromagnetic / Radar / Sonar | element/platform positions, DoA/TDOA/range-az types | **T** ("in production") | names mapped only | no algorithms |
| Hyperspectral | spatial grid, pixel location | **T** | `SpatiallyAdjacentTo` mapped, not derived | no cube-as-grid |
| UiDocument (127) | boxes, line boxes, transforms, z-order; frames page/viewport/screen | design; B1-B3 code edits pending the build gate | host-central (design) | README + rules guide only before this pass |

## 2. Crosswalk: modality x MCP x cross-links x graph-block geometry

| Modality | MCP(s) | Cross-modality links actually built | Geometry reaches graph block |
|---|---|---|---|
| Text | web-mcp, shared-context | hook (model-driven) linker only | no |
| Code | code-mcp, code-graph-mcp, git-mcp, terminal-mcp | declared only | no |
| Image | visual-mcp (yolo/depth/segment/shape/pose/scene_graph) | declared; F.3 spatial real | yes, own enum, `Image` tag, no InFrontOf |
| Audio/Sound/Video/Math/EEG | none (ffmpeg via terminal for video) | none | no |
| Chemistry/DNA/Proteomics | chemistry/dna/proteomics-mcp | none | graph block, `Unknown`, no coordinates |
| 3D | bridges (blender), threed-mcp, mesh-mcp | none | mesh block `Unknown` |
| CAD | mesh-mcp `cad_step_info` | none | no |
| Geospatial | geospatial-mcp | none | graph block `Unknown` |
| Depth | via visual-mcp | none | InFrontOf not persisted |
| Web/UI/mobile | browser, farm, android | none | stopgap JS blocks, `Unknown` |
| Other sensors | none | self-loops | no |

How modalities work with each other and with tools today:
- A modality run does not trigger tool calls. Tool routing (`tool_routing.rs`,
  `OZONE_TOOL_ROUTING=1`, off by default) keys on step text, picks at most 2
  tools, injects untrusted results into stage-7 prompts. The affinity guide's
  `autostart` modes and `modality:<name>` capability tags are specified, not
  built.
- Content enters a modality by `detect_modality` (path hints, extension, MIME,
  sample) and file_link. `/capture/ingest_image` is the only direct ingest
  route; a second (`/capture/ingest_uidocument`) reuses its gate pattern.
- Guests-on-owner (run-vs-tools doctrine): image runs first; YOLO/depth are
  tools that extend the graph. This works only because visual-mcp runs real
  models; the owner's own run is currently constant output (0.1).
- Missing cross-modality tool chains: video -> frames -> image; Blender render
  -> image; code -> terminal/git routing; sensor modalities have no tool face.

## 3. Target model (from SPATIAL_GEOMETRY_LAYER.md, refined by the audits)

1. **Frames**: one registry of named frames with parent transforms (page,
   viewport, screenshot px, screen px, image px, world, CRS, sensor array,
   time axis for audio/video). Adopt depth's `CoordinateSystem` + 4x4 as the
   seed (it is the most complete).
2. **Primitives**: Point, Segment/Polyline, Curve, Box (AABB/OBB), Polygon/
   Region(mask), Grid (raster/pressure/thermal/hyperspectral), Mesh/Volume,
   plus **Span** (1-D: text offsets, code lines, audio time) so non-spatial
   modalities have a place in the same layer.
3. **One relation vocabulary** in the host enum and `mcp_graph`: Above, Below,
   LeftOf, RightOf, Overlaps, Contains, Within, Touches, NearTo, InFrontOf,
   Behind, Aligned, Supports. Replace "reject the whole block" with
   per-edge rejection + a recorded note.
4. **Geometry field** on nodes (`geom`, `frame`, optional `transform`),
   readable at the write choke point (`src/zsei/mod.rs:~191`), so the host
   computes relations centrally and across graphs.
5. **Cross-graph links**: `Renders`, `Locates`, `DepictsSameEntity`, and
   registration edges (`RegisteredTo` with the transform: thermal<->image,
   depth<->image, EEG<->head, IMU<->world).
6. **Honest values only**: a modality with no real values emits no geometry
   and a visible `no-geometry` reason; fabricated constants (image analysis,
   chemistry structure, video track) are marked or removed.

## 4. Per-modality completion path (what makes each geometrically complete)

| Modality | To be complete |
|---|---|
| Image | decode pixels in the pipeline (image crate) for real dimensions/format/colour/EXIF; persist NearTo/Supports/InFrontOf; mask/shape nodes with edges; pixel frame + scale; `Locates` to UiDocument |
| Depth | PCD/PLY/LAS readers; back-projection of image depth via intrinsics (scale from a reference or stereo); pixel<->point link; persist InFrontOf |
| 3D | scene readers (glTF/OBJ/Blend via bridge); call bridge tools for real transforms; BVH/octree; geometric (not LLM) relations; frame = world |
| CAD | STEP/IGES reader (OpenCascade or trimesh path); sketch/DXF import; exact B-rep topology; AABB relations into the shared layer |
| Kinematics | URDF/MJCF parse; link geometry from CAD/3D; shared world frame |
| Geospatial | geometry-derived relations everywhere; implement the spatial index; link network `geo_location` |
| Chemistry / Proteomics | RDKit conformers; PDB/mmCIF `ATOM` parse; contact distances; frame = molecule; chem<->3D link |
| Video | real frame extraction (ffmpeg), tracks as images over time, motion geometry |
| Text | PDF/DOCX layout (page boxes, reading order) as geometry; tie spans to a frame |
| Code | columns; embedded HTML/CSS/script -> UiDocument link |
| Audio/Sound | time-frequency grid; localization algorithms (TDOA/beamforming) that fill `SourceLocalization`; sensor-frame -> geo transform |
| EEG/BCI | montage/head frame; electrode adjacency; source localization; EEG<->BCI frame |
| IMU/Haptic/Thermal/Hyperspectral | register frames (apply the 4x4, thermal<->image, cube as grid); derive neighbor edges from the grid |
| EM/Radar/Sonar | implement DoA/CFAR/beamforming/bathymetry or mark honestly absent; range-az -> world |
| UiDocument | build per README; dom-live feed with intrinsic boxes; android bounds |
| Network | geo-reference edge code or remove the claim |

## 5. Test rows (GEO series, wired into MASTER_TEST_GUIDE §20)
| id | check | expected |
|---|---|---|
| GEO1 | ingest a real farm screenshot into pipeline 102 | width/height/format/colour **derived from the file**, differs from 800x600 PNG constants for a 1920x1080 screenshot (RED today) |
| GEO2 | `analyze_image` source scan | no hard-coded `example_object` / constant result paths |
| GEO3 | send a graph block using LeftOf/Touches/Behind | edges accepted, or only the bad edge rejected with a note (RED today: whole block rejected) |
| GEO4 | graph block from dna/chemistry/geospatial/mesh/browser | modality tag correct, not `Unknown` |
| GEO5 | node with `geom`+`frame` in two modalities, same linked frame | one relation engine yields Overlaps/Contains across them |
| GEO6 | depth: image depth back-projected with intrinsics | metric 3D points; pixel<->point link; InFrontOf persisted |
| GEO7 | 3D scene via Blender bridge | real transforms in graph; relations from geometry not LLM |
| GEO8 | STEP file | real faces/edges, not format label |
| GEO9 | chemistry 3D | coordinates from RDKit, no fabricated 1.43 A pair |
| GEO10 | PDF page | per-block boxes and reading order |
| GEO11 | thermal<->image registration | `RegisteredTo` with transform; pixel mapping verified |
| GEO12 | sound localization | `SourceLocalization` filled by TDOA/beamforming or honestly absent |
| GEO13 | URDF/MJCF/SDF detect | unambiguous modality |
| GEO14 | no cross-modal self-loops | target ids resolve to real nodes in another graph |
| GEO15 | video | frames extracted, per-frame detections, motion geometry |

## 6. Order of work when the gate lifts
1. Fix the honesty gaps first (they invalidate evidence): image decode (GEO1/2),
   fabricated chemistry/video constants, and correct the earlier "real image
   analysis" claims in the evidence docs.
2. Shared layer: relation enum + `mcp_graph` per-edge rejection + `geom` field +
   central relation computation (B3 started the computation).
3. UiDocument (B1-B3) and its dom-live feed with intrinsic boxes.
4. Image `Locates`/`Renders` to UiDocument; depth back-projection.
5. 3D/CAD readers and bridge-fed transforms; geospatial index.
6. Sensors: register frames, then algorithms or honest absence.
