# Modality → Tool/MCP Affinity Guide (2026-10-30, operator directive)

> The doctrine (RUN-VS-TOOLS, parity manifest): content arrives → **the
> modality runs itself on it first** (its pipeline — the owning analysis →
> the base graph) → **tools/MCPs run ON TOP as guests**, extending the same
> graph (every addition ripples into the graph-native expansion). This guide
> is the per-modality affinity map: what exists TODAY, what's specified, and
> the new findings to look into. The affinity TABLE (the operational registry
> with `autostart: none|mcps|scan` rows) builds from this guide (registry §7).

## The affinity map (all modalities)

| Modality (pipeline) | Owning run | Tools/MCPs that run on top — TODAY | Specified / to look into |
|---|---|---|---|
| Text (100) | text pipeline: chunks, entities, topics, grammar edges | web tools, shared-context reads; extraction harvest | domorpher-core's extraction/instruction behind domorpher-agent (`domorpher_extract`) |
| Code (101) | code pipeline: functions/classes/edges | code-mcp (callgraph), code-graph-mcp (cross-file Python), git-mcp, terminal-mcp | cross-file edge build (T-C4's boundary); per-repo scan auto-run |
| Image (102) | image pipeline: bit depth, sRGB, composition (LIVE on EU) | **visual-mcp: yolo_detect/yolo_graph/yolo_segment/depth_graph** (graph-expanding; needs ultralytics); browser/farm screenshots via `/capture/ingest_image` (LIVE) | detection-model registry (§11.6 fan-out — N models, one graph); screenshot→ingest auto-run (X1a) |
| Audio (103) / Sound (110) | audio pipelines | — none registered today | whisper/analysis MCP (whisper_rs is already a pipeline dep) — NEW FINDING: the transcriber wants an MCP face |
| Video (104) | video pipeline | ffmpeg via terminal-mcp | frame-extraction → image-modality chain (video → frames → 102 fan-out) — NEW FINDING |
| Math (105) | math pipeline | — | proof-graph tools; the `.tex` routing (now canonical Math) makes pdf+tex unify here |
| Chemistry (106) / DNA (107) | chemistry/dna pipelines | — | domain MCPs (tool faces over the pipelines) — the 134 third-party tools may already carry some; audit against the affinity table |
| EEG (108) | eeg pipeline | — | same audit |
| 3D (109) | 3D pipeline | **bridges (:3210): blender_create_object/set_transform/bake/export**; threed-mcp (:3250) | Blender render → image-modality chain (BL9); glTF round-trip via the farm |
| Biology (111) / Proteomics / Haptic / Thermal / Depth / IMU / Geospatial / Electromagnetic / BCI / ParametricCAD / Kinematics / ControlSystems / NetworkTopology / Radar / Sonar / Hyperspectral | their pipelines | — none registered today | **THE AUDIT**: the fleet table lists dna/chemistry/proteomics/geospatial/mesh MCPs as "never recorded" — run their contract (MASTER_TEST_GUIDE L6) and fill this column for real |
| Web/pages (via FileReference + text) | the hosting modality's run | **browser-mcp (:3280), browser-farm-mcp (:3281), domorpher-agent (:3283 — skeleton)** | the google.com DOM-field harvest: domorpher-core turns the DOM into the traversable field |
| Any (cross-cutting) | — | terminal-mcp, git-mcp, web-mcp, ozone-shared-context (16 coordination tools), security-mcp, p2p-bridge | every tool call is modality-tagged at registration so the affinity table can key on it |

## The three operational rules

1. **Modality first, tools second.** A tool result that wants graph presence
   enters through the hosting modality's run (the ingest pattern) or as
   typed additions to its graph — never a side graph.
2. **The affinity table is declarative and per-install.** Rows:
   `{modality, tool, autostart: none|mcps|scan}` — `mcps` boot-spawns the
   modality's MCPs (supervisor pattern), `scan` auto-runs the tool over new
   un-analyzed content (X11: drop a PNG → yolo_graph, hands-free).
3. **Everything through `/mcp/call`.** Auto-run or operator-invoked, the
   call is metered, gated, rippled — the affinity table never creates a
   bypass.

## New findings to look into (captured, not built)

- **F-NEW-1**: the fleet's dna/chemistry/proteomics/geospatial/mesh MCPs have
  never been contract-tested — the affinity table's biggest empty column.
- **F-NEW-2**: audio/sound have NO tool face at all; whisper_rs in the
  pipeline deps suggests a transcriber MCP is near-free.
- **F-NEW-3**: video → frames → image-modality fan-out is the natural
  multi-modal chain (one video, N image graphs, one scene graph).
- **F-NEW-4**: tool calls should STAMP their modality scope at registration
  (capability tags today: `platform:linux/x64`) — add
  `modality:<name>` tags so the affinity table and the ripple scoping can
  key directly.
