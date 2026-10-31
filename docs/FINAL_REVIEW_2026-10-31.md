# Final review — decisions closed, modality status, alignment, context flow, completion list (2026-10-31)

Operator direction: a final review of everything from this arc; finalize every
decision with no open designs; status of all modalities (full checklist: complete /
in progress / not started); are all modalities following the modality rules and
structure; how context already works with what exists and what is added; all gaps
and how it all works together.

Basis: four code audits (A1-A4, file:line cited in `GEOMETRY_GUIDE.md`), the
`MODALITY_EXPANSION_GUIDE` contract, the live registries (local and EU), and the
work landed today. **Nothing in this arc has been compiled or run.** Where this
document says "landed" it means "edited and parse-checked". The one authorization
still needed is lifting the no-build rule (section 9).

## 1. Decisions — closed (nothing left open)

| # | Decision | Final answer | Why |
|---|---|---|---|
| D1 | Domorpher | Archived and removed. Nothing is ported. Old code lives in `~/Projects/_archive/` for review only. | Operator: start fresh; most of it duplicated Ozone (taxonomy test) |
| D2 | Where web/app structure lives | One modality: **UiDocument, pipeline 127** | Boundary test: it owns node/edge types (Document, Element, Form, Field, StyleRule, ScriptRef...), so it is a modality, not a tool |
| D3 | UiDocument flavors | `html`, `css`, `dom-live`, **`svg` (moved from Image, confirmed)**, `android-xml`, `a11y-json`; `patterns` is a rule pack, not a flavor | Same engine, same graph shape, cross-flavor links (StyledBy, WebView hand-off) |
| D4 | Rules | Rule packs are DATA (`when/emit/confidence/source/rationale`), one pack per flavor; absence is honest; every emitted item records its rule id. Other modalities adopt the same format when they need deterministic rules | Precedent: jurisdiction and methodology JSON |
| D5 | Geometry | Intrinsic to nodes (`geom` + `frame`), traversable, spans graphs. It is a **layer**, not a modality | Operator; matches LOCALIZATION_TRAVERSAL_GUIDE |
| D6 | Who computes spatial relations | The **host, centrally**, at the ZSEI write choke point (`src/zsei/mod.rs`), reusing `mcp_graph::compute_spatial_relations`. Pipelines emit geometry only; CAD B-rep topology and 3D scene-graph parent/child stay in their own engines | Only the host sees more than one graph. Today the pass is wired only into the graph-block path (see gap G1) |
| D7 | Final spatial relation set | `Above, Below, LeftOf, RightOf, Overlaps, Encloses, Within, Touches, NearTo, InFrontOf, Behind, Aligned, Supports`. Structural `Contains` stays structural; geometric containment is `Encloses/Within` | Already partly in the enum (B3); completes the set |
| D8 | Cross-graph relation set | `Renders` (UiDocument<->Image), `Locates` (detection -> DOM element), `DepictsSameEntity` (Image<->3D/CAD/SVG), `RegisteredTo` (frame registration with transform: thermal/depth/IMU/EEG). Added to host `RelationType` (additions only) | None exists today; self-loops are removed, not repaired |
| D9 | Unknown relation names | `mcp_graph` rejects the **edge** with a recorded note, never the whole block | Today one unknown name discards the block |
| D10 | Engine relation names | Flip `relation_mode` default from `host` to `semantic` once D8/D7 names are accepted; until then the `host` mapping stays | Avoid lossy mapping |
| D11 | Frames | One frame registry with parent transforms; seed = depth's `CoordinateSystem` + `Transform4x4`. Frames: page, viewport, screenshot-px, screen-px, image-px, svg:<id>, world, CRS, sensor-array, time-axis | Depth has the most complete types |
| D12 | Primitives | Point, Segment/Polyline, Curve, Box, Polygon/Region, Grid, Mesh/Volume, **Span** (1-D: text offsets, code lines, audio time) | Lets non-spatial modalities sit in the same layer |
| D13 | Live DOM | A read-only in-page JS observer (full tree once, then MutationObserver deltas, boxes kept current), injected by the farm bridge; ingest via `/capture/ingest_uidocument`; deltas ripple via GraphEventHub | Operator: the DOM is live, not a picture |
| D14 | Acting on pages | Browser-Farm / android-mcp through `/mcp/call` (locked by default, allowlisted). No domorpher agent. Deciding is Ozone's reasoning | Operator |
| D15 | Python | `uidocument-py` (PyO3 wheel over the same Rust crate) is the farm-side library; no separate CLI (the pipeline binary is the CLI); built at the gate | One code path, farm and pipeline cannot disagree |
| D16 | Honesty rule for modalities | A pipeline either computes a value or emits it as **absent with a reason**. Fabricated constants are removed. Placeholder `success:true` results are forbidden | `analyze_image` returned constants; chemistry fabricated a structure; video returned a canned track |
| D17 | Image pipeline | Decode the file (header dimensions/format/bit depth, EXIF, real colour statistics); objects/masks/depth come only from visual-mcp; pixel frame recorded | Fixes the EU "real analysis" claim at its source |
| D18 | Tool routing | Keep `OZONE_TOOL_ROUTING` default off until the per-modality `autostart` and `modality:<name>` capability tags exist; modality runs do not call tools themselves — the orchestrator/reasoning does | Prevents a hidden second path around the gate |
| D19 | Language identification | Capability tags `lang:<node|python|rust|javascript>` and `kind:<mcp|pipeline|library|worker>` on every registration; native `language` field later | Done for the browser registrar; extend to all registrars |
| D20 | Build/test location | Compile and test on the EU VPS (11 GB, systemd); local host stays up for monitoring only | Local crashed four times under build load |
| D21 | Proposals / assistant | Notification is the default kind; Action only for whitelisted tools; cooldown + idempotent Proceed; route half applied | Reminders awaiting clarification are the common case |
| D22 | Secrets | Rotate the OpenRouter key (operator). Tokens only via env/EnvironmentFile, never logged | Standing rule |

## 2. The modality rules (what "follows the structure" means)

From `MODALITY_EXPANSION_GUIDE` §1 (boundary test: *modalities define the graph,
tools expand it*) and the build recipe, plus this arc's additions:

| Rule | Statement |
|---|---|
| M1 detect | One extension/MIME table in `src/types/modality.rs` is the single source |
| M2 register | Pipeline id 100-199 in `pipelines/index.json` + loadable via the registry gate |
| M3 contract | Independent crate, own `[workspace]`, `{data,context}` envelope via `--input`/stdin, one JSON out |
| M4 owns types | The modality owns node/edge types in Rust; tools never invent types |
| M5 persist | Output persisted through the write choke point as ZSEI containers (`graphs/<modality>_<id>.json`) |
| M6 links | Cross-modality links are real edges to nodes in another graph (not self-loops) |
| M7 geometry | Nodes with a place carry `geom` + `frame`; relations computed centrally |
| M8 honesty | Computed or absent-with-reason; no fabricated constants (D16) |
| M9 safety | Content from outside the system is labelled untrusted (X2) |
| M10 tool face | Optional MCP via `/mcp/call` (metered, gated, rippled) |
| M11 UI | Discriminator + colour + legend entry |
| M12 tests | Crate tests + MASTER_TEST_GUIDE rows |
| M13 run-vs-tools | The modality runs itself first; tools are guests that extend its graph |

## 3. Status matrix — all 28 modalities

Legend: ✔ done · ◐ partial/typed only · ✖ missing · ? unverified.
Columns: **M1** detect table · **M2** registered in index.json · **M3** CLI contract · **M5** persist code present · **Built** binary built (EU: all 27 deployed in `/opt/ozone-studio/pipelines`; local: not in a runnable path I could confirm) · **M8** real values (from the audits) · **M7** geometry present · **M6** real cross-links · **M10** MCP.

| Modality (id) | M1 | M2 | M3 | M5 | Built | M8 real values | M7 geometry | M6 links | M10 MCP | Overall |
|---|---|---|---|---|---|---|---|---|---|---|
| Text (100) | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ analysis (baseline proven) | ◐ spans only; PDF layout lost | ◐ hook linker only | web-mcp | **Complete for text; geometry gap** |
| Code (101) | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | ◐ line spans, columns mostly unset | ◐ declared | code, code-graph, git, terminal | **Complete; geometry n/a (1-D)** |
| Image (102) | ✔ | ✔ | ✔ | ✔ | ✔ | **✖ constants** (visual-mcp real) | ◐ boxes/masks via tools; F.3 relations real | ✖ | visual-mcp | **In progress — honesty fix first (D17)** |
| Audio (103) | ✔ | ✔ | ✔ | ✔ | ✔ | ? | ◐ time only | ◐ declared | none | In progress |
| Video (104) | ✔ | ✔ | ✔ | ✔ | ✔ | **✖ canned** | ✖ | ✖ | ffmpeg via terminal | Not started (shell) |
| Math (105) | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ (baseline proven) | – | ◐ counted 0 | none | Complete; links open |
| Chemistry (106) | ✔ | ✔ | ✔ | ✔ | ✔ | **✖ fabricated 3D** | ✖ | ✖ | chemistry-mcp | In progress |
| DNA (107) | ✔ | ✔ | ✔ | ✔ | ✔ | ◐ intervals real via MCP | ◐ linear only | ✖ | dna-mcp | Mostly complete for 1-D |
| EEG (108) | ✔ | ✔ | ✔ | ✔ | ✔ | ✖ stub channels | ◐ electrode types unused | ✖ | none | Not started (shell) |
| 3D (109) | ✔ | ✔ | ✔ | ✔ | ✔ | ✖ no scene readers | ◐ types; bridge gives real transforms but unused | ✖ | bridges, threed, mesh | In progress |
| Sound (110) | ✔ | ✔ | ✔ | ✔ | ✔ | ✖ localization never filled | ◐ array positions typed | ✖ self-loop | none | Not started (shell) |
| Biology (111) | ✔ | ✔ | ✔ | ✔ | ✔ | ? | ? | ✖ | none | Unverified |
| Proteomics (112) | ✔ | ✔ | ✔ | ✔ | ✔ | ◐ sequence only | ✖ no coordinates | ✖ | proteomics-mcp | In progress |
| Haptic (113) | ✔ | ✔ | ✔ | ✔ | ✔ | ✖ | ◐ typed | ✖ | none | Not started (shell) |
| Thermal (114) | ✔ | ✔ | ✔ | ✔ | ✔ | ✖ | ◐ grid typed | ✖ | none | Not started (shell) |
| Depth (115) | ✔ | ✔ | ✔ | ✔ | ✔ | ✖ no readers; relative depth only via visual-mcp | ◐ best types | ✖ | via visual-mcp | In progress |
| IMU (116) | ✔ | ✔ | ✔ | ✔ | ✔ | ◐ fusion types | ◐ orientation, no parent frame | ✖ | none | In progress |
| Geospatial (117) | ✔ | ✔ | ✔ | ✔ | ✔ | ◐ **real** haversine/PIP | ✔ best CRS handling; index declared | ✖ network geo link unwired | geospatial-mcp | Furthest along (geometry) |
| EM (118) | ✔ | ✔ | ✔ | ✔ | ✔ | ✖ "in production" | ◐ typed | ✖ self-loop | none | Not started (shell) |
| BCI (119) | ✔ | ✔ | ✔ | ✔ | ✔ | ✖ | ◐ positions unused | ✖ | none | Not started (shell) |
| CAD (120) | ✔ | ✔ | ✔ | ✔ | ✔ | ✖ STEP not parsed; AABB checks real | ◐ rich types | ✖ | mesh-mcp (structure only) | In progress |
| Kinematics (121) | ✔ (ambiguous w/ 3D) | ✔ | ✔ | ✔ | ✔ | ◐ **real** FK | ◐ poses | ✖ | none | In progress |
| Control (122) | ✔ | ✔ | ✔ | ✔ | ✔ | ? | – | ✖ | none | Unverified |
| Network (123) | ✔ | ✔ | ✔ | ✔ | ✔ | ◐ | ◐ geo_location optional | ✖ unwired | none | In progress |
| Radar (124) | ✔ | ✔ | ✔ | ✔ | ✔ | ✖ | ◐ typed | ✖ | none | Not started (shell) |
| Sonar (125) | ✔ | ✔ | ✔ | ✔ | ✔ | ✖ | ◐ typed | ✖ | none | Not started (shell) |
| Hyperspectral (126) | ✔ | ✔ | ✔ | ✔ | ✔ | ✖ | ◐ grid typed | ✖ | none | Not started (shell) |
| **UiDocument (127)** | ✔ (this arc, incl. svg) | ◐ migration code, uncompiled | ✔ | ✔ (via ingest route) | ✖ not built | ◐ rules written, unrun | ✔ designed in (live feed) | ✔ designed (Renders/Locates) | farm, browser, android | **In progress — code landed, nothing compiled** |

Reading the matrix: M1/M2/M3/M5 are uniformly satisfied (the structure is
followed everywhere). The real shortfalls are M8 (honest values), M7 (geometry),
M6 (real links) and M10 (MCP face) — i.e. **structure is aligned; content is
not**. Twelve modalities are typed shells (no real analysis).

## 4. UiDocument alignment check (does the new work follow the rules?)

| Rule | UiDocument |
|---|---|
| M1 | ✔ html htm xhtml css scss less svg; `text/html`, `text/css`; `.js/.ts` stay Code; raster stays Image |
| M2 | ◐ `ModalityType::UiDocument`, id 127, bootstrap index migration — uncompiled |
| M3 | ✔ own workspace, lib + bin `uidocument`, envelope via `--input`/stdin; `normalize_input` accepts host route shape |
| M4 | ✔ it owns Document/Region/Element/Form/Field/Link/Media/Frame/StyleSheet/StyleRule/ScriptRef; tools only supply content |
| M5 | ✔ persisted through the same function as MCP graph blocks, with ws/proj scope |
| M6 | ◐ designed: `Renders`, `Locates`, `DepictsSameEntity` are decided (D8) but not yet in `RelationType` |
| M7 | ✔ geometry passes through; host computes relations (but only on the graph-block path — G1) |
| M8 | ✔ absence emits a reason (`geom_absent`), no constants |
| M9 | ✔ page text `untrusted`; suspicious-instruction flag; scripts never executed |
| M10 | ✔ farm/browser/android tools; JS `graph` blocks are a stopgap to remove |
| M11 | ✔ UI discriminator/colour/legend (B3) — uncompiled TS |
| M12 | ◐ ~35 crate tests + rows UD/UE/SG written, none run |
| M13 | ✔ the modality runs first; farm is a guest |

## 5. How context works today, what is added, and the gaps

### Today (verified)
1. Content enters by `detect_modality` / `file_link` or an ingest route.
2. The modality pipeline runs through the registry gate and persists a graph as
   ZSEI containers through the **universal write choke point**
   (`src/zsei/mod.rs`), which emits a ripple (GraphEventHub) with scope keywords.
3. Free actors observe the ripple (SecurityFinding, CoordinationFinding,
   TaskFailed, CodeStaleness) and feed `amt_candidates` -> `amt_loop`.
4. Tools run through `/mcp/call` (metered, jurisdiction-gated, rippled, insight
   envelope); a tool's `graph` block is persisted by `mcp_graph` through the same
   choke point (<=5000 nodes, <=20000 edges).
5. Reasoning (pipeline 9, AMT, stages) reads context by **traversal** over
   `context.relationships`; consciousness windows observe; the assistant derives
   a feed. Nothing in this chain calls tools except the orchestrator/reasoning.

### Added in this arc (uncompiled)
- UiDocument modality + ingest route + engine + observer + wheel.
- `geom`/`frame` convention on node attributes; `compute_spatial_relations`
  (same-frame boxes, grid index, k=4 NearTo, edge cap with truncation note).
- Relations `LeftOf, RightOf, Within, Encloses` added and accepted by `mcp_graph`.
- `/mcp/call` scope mirror (workspace/project), McpFailureActor, proposals
  hardening, `decision_review` pending visibility fix.
- Ingest hook auth (`x-oz-internal`), JS graph blocks (stopgap), language tags.

### Gaps (each has an owner step in section 7)
- **G1** Spatial relations are computed only on the `mcp_graph` persistence path.
  Modality pipelines that persist directly (image `CreateGraph`, geospatial, 3D...)
  bypass them. Fix: move the pass to the ZSEI choke point (D6).
- **G2** `Touches, Behind, Aligned, Supports` and the cross-graph relations are
  not in `RelationType`; whole-block rejection still exists (D8/D9).
- **G3** `InFrontOf/Behind` need z-order in `geom`; the observer should send
  stacking order (paint order index) with each box.
- **G4** Frame registry does not exist; frames are free strings (D11).
- **G5** `modality_for` tags most graph blocks `Unknown`; derive the modality
  from the tool's registered `modality:<name>` tag (D18) and from UiDocument.
- **G6** Cross-modality links are model-driven only (`zsei/hooks.rs`); add the
  deterministic `Renders/Locates/DepictsSameEntity/RegisteredTo` producers.
- **G7** The image pipeline does not decode pixels (D17); EU ingest evidence
  needs re-running after the fix.
- **G8** Twelve modalities are typed shells (section 3); each needs real values or
  honest absence (D16).
- **G9** URDF/MJCF/SDF are listed under both 3D and Kinematics: pick one owner
  (Kinematics owns URDF/MJCF; 3D keeps SDF-as-scene only if it parses scenes).
- **G10** Tool routing is off and modality-blind (D18); the `autostart` and
  `modality:<name>` tag work is unbuilt.
- **G11** Registries drift between hosts (EU registrar copy stale; ingest token env
  not deployed; stale tool rows pruned locally only).
- **G12** The farm CC bridge script that injects the observer lives outside this
  repo; wiring it is an operator-side step (the Browser-Farm repo is not edited).

## 6. What is complete / in progress / not started (whole arc)

**Complete (edited, committed; all uncompiled):** domorpher archived/removed;
agent and registry rows pruned; UiDocument README, rules guide, spatial layer
and geometry guide; UiDocument engine (html 74 rules, css 11); python wheel;
JS observer; host wiring (modality 127, ingest, spatial pass, new relations,
UI, build script); proposals R1-R3 + route half; scope mirror; failure actor;
ingest-hook auth; graph-block stopgaps; language tags (browser registrar).

**In progress:** the `svg` flavor in the engine (fork B4) and the SVG detect move
(done in `modality.rs`).

**Not started:** android-xml and a11y-json flavors; patterns rule pack; frame
registry; cross-graph relation producers; spatial pass at the choke point; image
decode; all G8 shells; autostart/tag routing; EU deploy of anything from this arc;
every compile and test.

## 7. The completion list, in order (owners)

1. **Authorization:** lift the no-build rule for EU-only compiles (operator).
2. **Gate 0 (compile):** `cargo build` host on EU; `cargo test` uidocument crate;
   fix compile errors (scraper 0.20 API, enum matches, serde comparisons).
3. **Gate 1 (honesty, D16/D17):** image decode; remove fabricated chemistry/video
   constants or mark absent; re-run EU ingest rows (GEO1/GEO2).
4. **Gate 2 (shared layer):** relation set (D7/D8), per-edge rejection (D9),
   frame registry (D11), spatial pass at the choke point (D6), z-order in `geom`.
5. **Gate 3 (UiDocument live):** deploy engine to EU (unit WorkingDirectory +
   binary path), register 127, run UD1-UD12/UE1-UE10; wire the observer in the CC
   bridge script (operator-side); T-DOM-GOOGLE D1-D8 on the real google.com page;
   build the wheel and run UE9; flip `relation_mode` default (D10); remove JS
   graph-block stopgaps.
6. **Gate 4 (cross-modality):** `Renders/Locates` (page<->screenshot),
   `DepictsSameEntity` (svg<->raster), `RegisteredTo` (thermal/depth/IMU).
7. **Gate 5 (modalities, by value):** 3D readers + bridge transforms; CAD STEP
   reader; depth readers + back-projection; geospatial index; chemistry/proteomics
   coordinates; PDF layout; then sensors (algorithms or honest absence).
8. **Gate 6 (routing/registry):** `modality:<name>` tags, autostart, language tags
   on all registrars, registry persistence verification (P8).
9. **Operator items:** rotate the OpenRouter key; systemd watchdog for the local
   host; decide on the Browser-Farm commit b603436 / `tools/bridges` gitlink; push
   the local commits.

## 8. Test coverage map (rows defined, none run)
UD1-UD17, UE1-UE10, SG1-SG10, GEO1-GEO15, LANG1-7, SC1-4, MF1-6, PR1-4, HK1,
GB1, RG1, T-DOM-GOOGLE D0-D8 (MASTER_TEST_GUIDE §17-§22 and the review docs).

## 9. What is needed from the operator now (not design questions)
1. Permission to compile and test on the EU VPS (Gate 0).
2. Confirmation that the Browser-Farm CC bridge script may be edited outside this
   repo to inject the observer (Gate 3).
3. The key rotation and the Browser-Farm/`tools/bridges` decision (section 7.9).
