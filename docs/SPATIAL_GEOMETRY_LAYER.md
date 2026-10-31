# The spatial geometry layer — geometry is intrinsic, shared, and spans graphs (2026-10-31)

Operator direction: geometry is not "requested". It is there, it is traversable,
and the relationships it creates span graphs. This must hold for every
modality: DOM, CSS styling, images and pixels, lines and shapes and drawings,
3D, CAD. Question: after this, is it still one modality for all, or not?
Design only. Nothing built or run.

## 1. Answer first

**Not one modality for all. One geometry layer for all.**
- Modalities split on *data type and engine* (a mesh solver and an HTML parser
  share nothing). That stays: UiDocument, Image, 3D, CAD, Depth, Geospatial,
  Chemistry/Proteomics, EEG/radar/sonar, Video, ...
- Geometry is *not* a modality. It is a **layer every modality feeds**: a shared
  vocabulary of frames, primitives and spatial relations, so relationships can
  span graphs (a DOM button overlaps a detected region in its screenshot; a CAD
  part touches an assembly face; a 3D object is depicted by an image region).
- **UiDocument stays one modality** (html, css, dom-live, android-xml, a11y-json)
  and gains an `svg` flavor (see 4): they are all "a tree of elements with
  geometry and style", one engine.

## 2. What exists already (grounded in the repo)
- Spatial relations are real: F.3 on image detections (`Above/Below/LeftOf/RightOf/NearTo/Supports`, persisted as `ImageEdgeType`); the host's `mcp_graph` hook accepts `Above/NearTo/InFrontOf/Overlaps`; `LOCALIZATION_TRAVERSAL_GUIDE` already states the principle ("not an image-specific trick; any modality whose referent has a spatial layout", relations "computed from the actual geometry, never guessed").
- Known gaps (CAPABILITY_EXPANSION_REVIEW): `InFrontOf` computed but never persisted for image; no `DepictsSameEntity` link between image and 3D; PDF/text pages lose their layout geometry; relations are computed per tool, not centrally.
- 3D has `Transform4x4`; CAD has `InterfacingFaces`; geospatial has coordinates.

## 3. The layer (four parts)

1. **Frames.** Every geometric thing lives in a named coordinate frame with a
   transform to a parent frame: page CSS px, viewport, screenshot pixels
   (= viewport x devicePixelRatio, offset by scroll), Android screen px, image
   pixels, world (3D/CAD), CRS (geospatial), sensor array. Linking frames is
   what lets relations cross graphs: the screenshot frame is *defined* from the
   viewport frame.
2. **Primitives.** `Point`, `Segment/Polyline`, `Curve`, `Box` (AABB/OBB),
   `Polygon/Region` (outline or mask), `Grid` (raster pixels), `Mesh/Volume`.
   2D or 3D recorded. Every node that has a place carries `geom` + `frame`.
3. **Relations.** One vocabulary for all: `Above/Below/LeftOf/RightOf`,
   `Overlaps`, `Contains/Within`, `Touches`, `NearTo`, `InFrontOf/Behind`,
   `Aligned`, `Supports`. Each edge records frame, metric and tolerance.
   Computed from geometry by one engine, never guessed; absent when a node has
   no geometry (no layout ran).
4. **Cross-graph links.** `Renders` (UiDocument <-> Image: frame mapping),
   `DepictsSameEntity` (Image <-> 3D/CAD, the open gap), `Locates` (a
   detection <-> the DOM element under it). With frames linked, spatial
   relations are also computed *between* graphs.

### Where the engine lives (recommended)
- **Host side at the write choke point.** Pipelines and MCPs emit geometry
  (`geom`, `frame`) on nodes; the host's universal ZSEI write / `mcp_graph`
  hook computes spatial relations centrally. Only the host sees more than one
  graph, so cross-graph relations are only possible there. It also ends the
  per-tool re-implementation (JavaScript `graph` blocks, per-MCP code).
- **Pipeline side stays for exact intra-modality topology** that needs the
  modality's own engine (CAD B-rep adjacency, 3D scene graph parent/child).
- **Scale**: a spatial index (grid/R-tree) gives exact `Overlaps/Contains` and
  bounded `NearTo` (k nearest) instead of O(n^2). The existing 5,000-node /
  20,000-edge cap stays; truncation is recorded in the graph, not hidden.

## 4. Per modality: what geometry it contributes
| Modality | Primitives and frame | Source of the geometry | State |
|---|---|---|---|
| UiDocument html/dom-live | boxes, line boxes, transforms, clip, z-order/stacking; frame = page, viewport | the live DOM via the in-page observer (box captured with the node, kept current by resize/intersection/scroll/mutation); computed style | design |
| UiDocument css | position/size/transform/overflow rules *determine* geometry; paint properties (color, radius, shadows) are attributes of the region | cascade over the document | design |
| UiDocument android-xml | `bounds` rect; frame = screen px | uiautomator dump | dump exists, no graph |
| UiDocument svg (proposed flavor) | exact lines, polylines, arcs, Beziers, polygons, groups, transforms ("drawing 101": point, line, shape, area) | the XML tree itself, no detection needed | SVG is listed under Image today |
| Image | pixel grid; detected boxes/masks; edges/lines/shapes; frame = image px | pipeline 102 + detection registry (YOLO, depth, grounding) | F.3 live |
| Video | image per frame + tracks over time | pipeline 103-ish | stub-level |
| Depth / point cloud | per-pixel depth, 3D points; links image pixels to 3D | depth models | registry live |
| 3D | meshes, scene graph, `Transform4x4`, bounding volumes; frame = world | Blender/engine scene | proven |
| CAD | sketch geometry (lines, arcs, circles), B-rep faces/edges, assembly mates | CAD parse | compiles, no persistence |
| Geospatial | point/line/polygon in a CRS | coordinates | registered |
| Chemistry / proteomics | atom coordinates and bonds in 3D | structure files | registered |
| EEG/BCI/radar/sonar | sensor-array topology | sensor layout | registered |
| Text / code | none intrinsic. A *rendered* page or a PDF has line/block boxes; today PDF layout geometry is lost | UiDocument (rendered) / PDF layout | gap |

## 5. How the cross-modality examples read
- A Google page: `searchbox` (box, page frame) `Within` `Region:header`;
  `Renders` links the Document to the screenshot; a YOLO/text detection in the
  screenshot (pixel frame) `Overlaps` the searchbox box after frame mapping, so
  the detection `Locates` the DOM element.
- An SVG logo on that page: the `<svg>` element is a UiDocument node with a box,
  its `<path>` children carry exact geometry; the same logo rasterized in the
  screenshot is Image pixels; `DepictsSameEntity` ties them.
- A CAD part inside a 3D scene: CAD faces `Touches` each other (exact
  topology, CAD engine); the part's bounding volume `Within` the scene object
  (shared relation).

## 6. Consequences for earlier decisions (corrections)
- dom-live: geometry is captured **with** every node (not requested on demand)
  and kept current; static HTML with no layout engine honestly has none.
- The JavaScript `graph` blocks in `browser_snapshot` / `android_ui_snapshot`
  stay a stopgap; the end state is geometry on nodes + central relations.
- SVG: move from Image's extension list to UiDocument flavor `svg` for the
  exact-geometry tree; raster stays Image. (Operator to confirm.)
- PDF/DOCX layout geometry becomes a text-modality enrichment later.
- `InFrontOf` persistence and `DepictsSameEntity` are no longer separate small
  gaps; they are part of this layer.

## 7. Tests
| id | check | expected |
|---|---|---|
| SG1 | every node from the dom-live feed carries `geom` + `frame` | no spatial-less element when a layout engine ran |
| SG2 | static HTML with no layout | no spatial edges (absent, not guessed) |
| SG3 | scroll / resize / mutation | box updates as deltas; relations recomputed for touched nodes only |
| SG4 | host computes `Overlaps/Contains/NearTo` from `geom` for UiDocument, Image and 3D nodes with one engine | identical edge semantics across modalities |
| SG5 | screenshot + DOM of the same page | `Renders` frame mapping is correct (devicePixelRatio, scroll); a detection overlapping the search box `Locates` that element |
| SG6 | SVG with known path | exact line/polygon geometry, no detection used; rasterized copy `DepictsSameEntity` |
| SG7 | CAD assembly + 3D scene | CAD topology edges from CAD engine; bounding-volume relations from the shared layer |
| SG8 | 1,000-element page | spatial index; edges bounded; truncation recorded |
| SG9 | `InFrontOf` | persisted for image (depth) and DOM (z-order) |
| SG10 | node without geometry | produces no spatial edge and a visible `no-geometry` reason |
