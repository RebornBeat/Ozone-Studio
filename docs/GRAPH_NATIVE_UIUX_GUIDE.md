# The Graph-Native UI/UX — Ozone GraphKit

> The doctrine, the verified state of the UI today, and the full specification
> of **GraphKit** — Ozone-Studio's own purpose-built UI library for its
> graph-based engine. Written 2026-10-23 (zcode) from a full read-only review
> of the real UI tree (every claim evidence-checked against source; see the
> verification appendix). **Nothing here is built yet** — this is the
> finalized design the next UI waves implement against. Status: **GO given
> 2026-10-23; steps 1–2 COMPLETE + the GraphKit core library LANDED
> (ui/src/graphkit/ — 6 files); steps 3(beta)/6/7/8 remain** — see §5's
> status column. Supersedes the UI-shape portions of UI_UX_FORK_PLAN.md /
> WORKSPACE_UI_REVAMP_PLAN.md / UI_UX_REACHABILITY_GUIDE.md (their completed
> B/W waves remain historical record — the bones they built are inventoried
> in §2 and become GraphKit's foundation, not scrap). Edits only — NO build
> yet, per the standing ruling; compile-clean is UNPROVEN until the build pass.

---

## 1. The doctrine (the operator's ruling, formalized)

**Everything the UI shows IS the graph.** Projects, workspaces, files,
methodologies, blueprints, AMT branches, tasks, capture stores, coordination
events, P2P peers, and every modality's nodes and edges — all of it is
containers and edges in one ZSEI store. Therefore:

1. **One store, many lenses.** Every view is a *projection* of the same graph —
   never a private dataset. An engine view, the file explorer, the task board,
   and the fabric view are four lenses over one substrate, switchable without
   losing place.
2. **No silos.** A view that fetches only its own slice and renders it without
   its cross-links is wrong even when it works — because in this system the
   cross-links ARE the meaning (a text entity matters because it DescribesCode;
   a file matters because it is linked into the graph; a modality graph matters
   because SimilarTo ties it to its siblings).
3. **Navigation is the spine.** The system already has `navigation.ts` with
   `graph-node / container / task / code-file / tab` targets. GraphKit's rule:
   every rendered entity links everywhere it exists — "view in graph", "open in
   engine", "open file", "open container" — one navigation spine, no dead ends.
4. **Live by default.** The graph is alive (ripple events, node events). Views
   invalidate-and-refetch today; GraphKit extends the same WebSocket singleton
   to per-node construction pulses and task progression — always REAL captured
   signals (no fabricated progress percentages — standing directive).
5. **The library is ours.** Not three.js, not recharts — a small, specific
   React component + data library built for THIS engine's shapes: containers,
   27 modalities, typed edges, cross-modal relations, capture stores, tasks
   with per-step state. Generic graph libs fight these shapes; GraphKit is
   them. (three.js/recharts may still be used INSIDE specific lenses later —
   a 3D viewport, a metrics chart — but they are leaf renderers behind
   GraphKit's contracts, never the architecture.)

---

## 2. What exists today — the verified inventory (the bones worth keeping)

Read directly from `ui/src` this pass; line counts real.

### 2.1 The data + event plumbing (the best bones)
| Piece | What it does | Verdict |
|---|---|---|
| `graphViewData.ts` (242) | `loadGraphData(projectId, status, {modality})` — fetches project's ModalityGraph children, converts to a unified node/edge model, live invalidation via WS | **Keep — becomes GraphProvider's fetch core.** But its modality discriminator recognizes only 6 path prefixes (§3.1) |
| `graphEventClient.ts` (539) | Process-wide WS singleton; TWO streams: graph events (invalidate) + node events (construction) | **Keep as-is** — the shared-singleton pattern is exactly right |
| `graphViewTypes.ts` | Unified `GraphViewNode/Edge` model: `${modality}:${containerId}:${nodeId}` composite ids, modality, isReal, raw | **Keep — GraphKit's node model** |
| `graphRenderers/` | Per-modality classifier plugins (code/math/text/image Nodes+Edges), `overlays/` (crossModal C8, governance, lineage), `NodeShapeSvg`, `MODALITY_COLOR` (all 27 modalities colored) | **Keep — the plugin pattern IS the engine-lens pattern.** The color system already covers all 27 |
| `navigation.ts` | 5 target kinds, pending-navigation queue, `takePendingNavigation` | **Keep — the spine; extend, never replace** |
| `services/store.ts` | zustand `useOzoneStore` (tasks, quota, ledger, UI state) | **Keep — GraphProvider sits beside it, same pattern** |

### 2.2 The views
| Surface | Files | State |
|---|---|---|
| Engines panel | `EnginesPanel.tsx` registry: 8 groups (code, math, text, image, 3d, audio-video, geospatial, CAD); 11 engine components (~2,900 lines) | Each engine = honest, hand-rolled, modality-filtered fetch + internal tree/table render. **Siloed by design** (§3) |
| Graph view | `GraphView.tsx` (601) + overlays | The one multi-modal canvas; the only consumer of `containerRelations` (cross-modal SimilarTo overlay, live-measured honest rendering) |
| Fabric | `FabricView.tsx` | Cluster summaries + `SimilarLink` cross-container links — **closest existing thing to a unified lens** |
| Files | `WorkspaceBrowser` (graph-native FileReference/URL/Package containers, honest empties), `FileReferenceViewer`, `CodeViewer`, `FileEditor`, `MathViewer`, `VersionHistory`, `CodeNodeLinks` | Graph-native data, but a **separate tab silo** — not woven into graph/engines |
| Hierarchy | `AmtTreeView`, `ForkLineageView`, `JurisdictionHierarchy` | AMT/lineage lenses — good bones |
| Workspace | `ProjectWorkspaceViewer` — sections: files / hierarchy / engines (+ graph, fabric, context…), each with SUBS | The shell to revamp (§4.7); mirrors ENGINES registry by hand (two books, §3.5) |
| Elsewhere | OrchestrationStatusPanel (stage rows, now incl. stage 14), TaskDetailPanel, OrderPanel, CreationActivity, LivingNetworkView, DiscoveryEffects, ThemeArea, MetaPortion, capture/coordination/search/task views | Real, honest, islanded |

### 2.3 The honest strengths to preserve
- **No-fabrication discipline everywhere**: honest loading/error/empty states, `isReal` edge classification, the 3D view's "no total known — real counts only" live indicator, FabricView's "29 truly cross-modal of 158" honesty.
- **Design system exists**: tailwind + CSS vars (`--color-*`), MODALITY_COLOR, PanelShell chrome, `lucide-react` icons available (underused).
- **Node events**: 3D's `justTouched` pulse (ThreeDEngine) is the correct pattern for construction feedback — currently the only consumer of the node-event stream (24 more producers sit unbuilt in the tree).

---

## 3. The diagnosis — why the current UI/UX is structurally wrong (verified)

The operator's framing — "each is separated as if it is not all graph-based" — is exactly what the code shows. Six concrete breaks:

1. **The modality discriminator only sees 6 of 27 modalities.**
   `modalityFromObjectStorePath` (graphViewData.ts:66-82) matches only
   `code_ / math_ / text_ / image_ / 3d_graph_ / cad_` prefixes. Every other
   modality's graphs (geospatial, radar, eeg, biology, chemistry, dna, audio,
   video, …) are dropped before ANY view can see them — GraphView's "all" is
   6 modalities deep. **GeospatialEngine is structurally broken by this**: it
   filters `{modality:"geospatial"}` which the discriminator can never return
   (backend writes `graphs/geospatial_{id}.json`, geospatial/main.rs:1293) →
   the view can only ever render its empty state, real data or not.
2. **Engines are fetch silos.** Every engine view calls
   `loadGraphData(projectId, …, {modality})` — its own fetch, its own
   subscribe, its own copy of the graph in memory. Ten views open = ten
   fetches of overlapping data. No engine renders anything OUTSIDE its
   modality — a text entity that `DescribesCode` shows no trace of the code
   node it points at; the "view in graph →" link is the ONLY bridge.
3. **Cross-modal knowledge is captured but mostly unrendered.**
   - The C8 crossModal overlay (real, live-measured: 158 SimilarTo relations,
     29 truly cross-modal) renders ONLY inside GraphView.
   - `GET /cross-modal-pairs` (84+ real pairs, backend route live) has **zero
     UI consumers**.
   - Within-graph "cross-modal" edge classes (DescribesCode/…) are defined in
     textEdges.ts but the text pipeline never actually sets them true
     (hardcoded false at creation) — a data gap the UI cannot paper over.
4. **Files are a silo, not a layer.** WorkspaceBrowser is graph-native and
   honest — but it lives behind a "Files" section. From a graph node you
   cannot reach the file that produced it (no link strip); from a file row
   you cannot jump into the engines over its graph (CodeNodeLinks is
   code-only). The FileRef-into-graph linkage exists in the STORE; the UI
   just never draws it as one fabric.
5. ~~Two engine registries~~ **CORRECTED 2026-10-23: already single-source** —
   ProjectWorkspaceViewer imports `ENGINES` from EnginesPanel and derives its
   sub-tabs (PWV:47 + :90 `ENGINES.map(...)`); the 10-18 fix landed. The
   original review overstated this; captured for the record. (GraphKit's
   registry keeps it single-source going forward.)
6. **Icons/indicators are minimal and inconsistent.** Task states, loop
   activity, capture presence, peer state, gate outcomes have no unified
   indicator language; lucide-react is available but barely used; progress is
   text ("42%") not visual; the one great indicator (3D's construction pulse)
   is locked inside one view.

**Root cause, one sentence:** the UI was built view-first (each view fetches
its slice) instead of graph-first (one substrate, many projections) — even
though the backend is graph-first end to end.

---

## 4. Ozone GraphKit — the library specification

`ui/src/graphkit/` — a small, specific library. Five layers, bottom-up. Every
layer extends an existing bone (§2) rather than replacing it.

### 4.1 Layer 1 — Data core: `GraphProvider`

The single source of truth for a project's graph in the UI.

```tsx
// graphkit/GraphProvider.tsx  (zustand store + one fetch + one WS subscription)
interface GraphKitState {
  // The substrate — ALL modalities, not a filter
  nodes: Record<string, GKNode>;            // unified model (graphViewTypes extended)
  edges: Record<string, GKEdge>;            // in-graph edges
  crossLinks: GKCrossLink[];                // container-level SimilarTo/[13.link]RelatedTo,
                                            // resolved endpoints (C8 overlay logic promoted here)
  containers: Record<number, GKContainer>;  // every container under the project, typed
  files: GKFileRef[];                       // FileReference/URL/Package containers as first-class
  tasks: GKTaskRef[];                       // task rows w/ per-step state (from /task list)
  status: 'loading'|'ready'|'error'|'empty';
  lastEventAt: number;                      // ws heartbeat for presence indicators
}
```

- **One fetch** per project: project container → children (all types, not
  just ModalityGraph) → modality graph contents (prefix table FIXED to all
  27 + a generic `graphs/<name>_` fallback so a new modality can never be
  invisible again) + container relationships + file refs + cross-modal-pairs
  route (finally consumed).
- **One subscription**: the existing graphEventClient singleton; graph events
  → scoped refetch (unchanged semantics); node events → per-node pulse
  broadcast (4.4).
- **Selectors, not copies**: views select slices (`useGraphKit(s =>
  engineLens('code', s))`). Ten open views = one fetch.
- Existing `loadGraphData`'s conversion + discriminator move here verbatim
  (with the prefix fix); engines stop calling fetch directly.

### 4.2 Layer 2 — Rendering primitives

Hand-rolled (SVG/CSS), consistent, ours. No new heavy deps; three.js/recharts
may sit behind these contracts later as leaf renderers only.

| Primitive | Purpose |
|---|---|
| `<GraphCanvas layout="force\|radial\|tree\|grid">` | The one canvas component; layout prop, not N canvases. Renders via the existing classifier plugins (graphRenderers/) so per-modality shapes/colors keep working |
| `<GKNode>` / `<NodeCard>` / `<NodeInspector>` | Every entity renders through ONE node card contract: icon (4.4), modality badge (MODALITY_COLOR), title, isReal, confidence-when-real, raw fields (engine lenses add their own detail rows) |
| `<EdgeLine>` / `<EdgeInspector>` | Typed-edge rendering with the existing edgeClass honesty (real/derived/inferred) |
| `<CrossLinkStrip entity={id}>` | THE silo-killer: for any node/container/file, lists its cross-links (SimilarTo/RelatedTo/DescribesX/FileRef) as clickable chips that navigate (4.3). Appears in every inspector, every engine, the file viewer |
| `<ModalityWheel>` | The 27-modality radial switcher — one component, used by workspace header, GraphView filter, and Fabric (replaces per-view modality lists) |
| `<GraphCanvas layout="grid\|radial\|tree">` | ✅ LANDED (`graphkit/GraphCanvas.tsx`): deterministic SVG layouts, MODALITY_COLOR fills, edgeClass dash honesty, selection ring; a physics lib may sit behind this contract later as a leaf renderer |
| `<HonestState>` | The unified loading/error/empty contract every view already follows informally — codified (loading spine, error with real message, empty with WHY it's empty + how to make it non-empty) |

### 4.3 Layer 3 — Navigation spine (extend, never replace)

- Keep `navigation.ts`'s 5 kinds; add `engine` (open modality lens @ node) and
  `file-ref` (open file viewer @ container). Every CrossLinkStrip chip, every
  NodeCard, every task row navigates through the spine — no view-local
  linking.
- **Breadcrumbs from the graph**: the workspace header shows
  `Project › Section › Lens › Selection` driven by navigation state, so a
  cross-link jump is always reversible.

### 4.4 Layer 4 — Indicators & iconography (the flourish)

All indicators render REAL captured values only (standing law: no fabricated
confidence/progress). `lucide-react` throughout; one mapping table:

| Indicator | Source (real) | Rendering |
|---|---|---|
| `TaskStateIcon` | task.status + steps[].status | queued(ellipse) → running(loader + per-step pip row) → paused / interrupted(octagon, "won't resume on its own") → completed(check) / failed(x, error text on hover) |
| `StepPips` | step statuses from the task record | a row of pips, one per step, filled by state — progression at a glance, no percentage invented (progress% shown only when the task record carries one) |
| `ConstructionPulse` | node-event stream (graphEventClient.onNodeEvent) | the 3D view's proven `justTouched` highlight, promoted to a primitive — any lens can pulse nodes as its modality constructs them (24 producers awaiting build) |
| `LoopPulse` | loop log/presence (which loops ran recently — from real log lines/capture rows) | small heartbeat dot on workspace header: amt/meta/i-loop/assistant chips, lit by last-real-activity |
| `CapturePresence` | S10–S13 stores non-empty | a "recorded" pip on views whose data has capture backing |
| `PeerState` | /network/status + paired-peers | paired(linked-dots) / connected-unpaired(link broken, honest: "connected, NOT trusted") / off |
| `GateState` | decision-review outcomes | Proceed(check) / ReviewPending(clock — with resolve-route once built) / Decline(shield-x) |
| `ModalityBadge` | MODALITY_COLOR | dot+label, the universal modality marker |
| `EdgeRealism` | edgeClass | solid real / dashed derived / dotted inferred — one legend everywhere (EdgeLegendFilters generalized) |

### 4.5 Layer 5 — Engine lenses (all 27, one pattern)

An engine becomes a **lens declaration**, not a fetch:

```tsx
// graphkit/lens.ts
const codeLens = defineLens({
  modality: 'code',
  nodeTypes: ['Function','Class','Import','Module', ...],   // shown
  focus: ['Calls','Contains','Imports'],                    // structural edges it trees/groups by
  detail: (node) => <CodeDetailRows node={node} />,          // engine-specific rows
});
```

- Lens chrome is shared: header (ModalityBadge + ConstructionPulse + HonestState),
  canvas area (GraphCanvas layout of choice), inspector (NodeCard + CrossLinkStrip +
  lens detail rows).
- The existing 8 engine views port as lenses; their tree-builders (e.g. 3D's
  `buildSceneTree`) become layout functions. The 19 modalities with views-but-
  no-reach (geospatial) or no views at all (radar, eeg, …) get a **generic
  lens** for free (typed node table + CrossLinkStrip + badge) — the fleet is
  covered day one, specialized lenses land incrementally.
- EnginesPanel and ProjectWorkspaceViewer both render from ONE registry export
  (kills the two-books defect).

### 4.6 Files woven in

- `GKFileRef` nodes appear in the substrate; `<CrossLinkStrip>` on a file
  shows its graph containers; on a graph container shows its files.
- WorkspaceBrowser keeps its graph-native list (it's right) but rows gain
  ModalityBadges (which graphs this file feeds) and engine-jump chips.
- The file viewer's inspector embeds NodeCard for linked nodes — file ↔ graph
  becomes one fabric, both directions, no separate "Files world".

### 4.7 The workspace shell revamp (one workspace, lenses not silos)

`ProjectWorkspaceViewer` becomes the single graph workspace:

```
┌ Workspace header: project · ModalityWheel · LoopPulse · PeerState · breadcrumbs ┐
│ ─────────────────────────────────────────────────────────────────────────────── │
│ LEFT RAIL (always): mini graph (GraphCanvas radial, current lens highlighted,   │
│   cross-links drawn) · task rows w/ TaskStateIcon+StepPips · recent containers   │
│ ─────────────────────────────────────────────────────────────────────────────── │
│ MAIN (lens area — tabs are LENSES over the same substrate):                      │
│   Overview | Graph | Fabric | Engines ▸ (27 lenses) | Files | Hierarchy |        │
│   Context (AMT/lineage) | Tasks | Capture | Coordination | Network               │
│   — switching lenses keeps selection & place (navigation spine)                  │
└──────────────────────────────────────────────────────────────────────────────────┘
```

- Sections stop owning data; they select from GraphProvider.
- The global panels (engines tab etc.) render the SAME lens components with a
  project picker — one component, two mounts (the canonical-surface law).
- Modernization is mostly RESTRAINT: consistent chrome (PanelShell), one
  spacing/type scale (tailwind tokens already present), indicators everywhere
  (4.4), and density from the graph (cross-link chips, badges, pips) doing the
  visual work — not decoration.

---

## 5. Migration order (edit-only compatible; each step shippable, no builds yet)

| # | Step | Status 2026-10-23 (zcode) |
|---|---|---|
| 1 | **Fix the discriminator** — all-27 single-source table + unknown-count | ✅ LANDED: `modalityFromObjectStorePath` now lives in `graphRenderers/defaults.ts`, DERIVED from MODALITY_COLOR's key set (all 27 prefixes verified against each backend's own format! string — incl. `3d_graph_`/`imu_`); graphViewData + crossModal both consume it (crossModal's own stale 3-prefix copy fixed too); `unknownModalityGraphs` counted in every fetch, never silently dropped. GeospatialEngine unblocked. |
| 2 | **Consume /cross-modal-pairs** | ✅ LANDED twice: the C8 overlay merges `GetCrossModalPairs` (via the existing zseiQuery path — it IS a ZSEIQuery variant) fail-soft into its relations, so pairs held by containers this session didn't fetch now render; AND the GraphProvider substrate joins the same route into its cross-link index. |
| 3 | **GraphProvider** | ✅ CORE LANDED: `ui/src/graphkit/graphProvider.ts` (zustand; one fetch: project → ALL children partitioned modality-graph/file-ref/other → contents → cross-links from relationships + pairs, both-direction reverse index; ONE shared WS subscription; node-event pulse counter substrate-wide; selectors). Views keep `loadGraphData` (which shares the same now-fixed conversion path — no behavioral break); view migration onto the provider is the next wave (GraphView + FabricView first). |
| 4 | **CrossLinkStrip + NodeCard** | ✅ PRIMITIVES LANDED (`graphkit/primitives.tsx` + `indicators.tsx`); retrofitting the 8 engine inspectors remains. |
| 5 | **Indicator set** | ✅ LANDED: TaskStateIcon / StepPips / GateState / PeerState (honest "connected ≠ trusted") / ModalityBadge / EdgeRealismLegend / ActivityPulse — all lucide names verified against the installed version; retrofitting existing panels remains. |
| 6 | **Engine lenses** | 🔶→✅ MOSTLY: registry + generic lens landed AND upgraded — genericLensFor now renders **GraphCanvas** (real canvas, selectable) above the typed table, and EnginesPanel gained an **All-modalities tab** (18 non-dedicated modalities reachable at this surface, honest per-modality table). Porting the 9 dedicated engines' bodies as lens renders remains (they keep working as-is meanwhile). |
| 7 | **Workspace shell revamp** | ⬜ next wave (view migrations 3–4 land first). |
| 8 | **Files weaving** | ✅ SUBSTANTIAL: substrate GKFileRef+linkedContainers (provider) + the **UI chip landed** — every WorkspaceBrowser file row carries a `graph →` chip (navigateTo container). Reverse direction (graph→files) rides CrossLinkStrip whenever views adopt it. |

Steps 1–2 are tiny; 3–4 are the heart; 5–8 flourish. None require backend
changes except consuming two existing routes.

## 6. What must NOT change (laws carried forward)

- No fabricated anything — indicators show real captured values or honest
  absence (the 3D pulse pattern; "no total known" honesty).
- One canonical surface per domain — lenses are components, mounted wherever
  needed, never duplicated.
- The WS singleton pattern (connect-shared, never per-view sockets).
- Honest empty/loading/error states with reasons (HonestState codifies, never
  waters down).
- The classifier-plugin pattern (graphRenderers) — lenses extend it, engines
  stop reimplementing it.
- Existing design tokens (CSS vars, MODALITY_COLOR, tailwind) — GraphKit
  consumes them; no second design system.

## 7. Verification appendix — evidence for every load-bearing claim

| Claim | Evidence |
|---|---|
| No graph/chart/3D deps | ui/package.json deps: react, zustand, lucide-react, qrcode, electron-log, grpc-js only |
| 6-of-27 discriminator + geospatial break | graphViewData.ts:66-82 (prefix list); GeospatialEngine.tsx:174 (`modality:"geospatial"`); geospatial/main.rs:1293 (`graphs/geospatial_{id}.json`) |
| Engines fetch-filter per view | every engine component's `loadGraphData(..., {modality})` call (ThreeDEngine:252 et al.) |
| C8 overlay real + only in GraphView + 158/29 measured | graphRenderers/overlays/crossModal.ts header; GraphView.tsx:579 sole pass-site |
| /cross-modal-pairs unconsumed | repo grep: zero UI references to the route |
| text cross_modal never true | textEdges.ts:33-34 comment (hardcoded false at creation) |
| Two registries | **CORRECTED**: PWV.tsx:47 imports ENGINES from EnginesPanel; :90 derives — single-source already |
| Node events one consumer | ThreeDEngine.tsx:312 sole onNodeEvent call |
| MODALITY_COLOR covers 27 | graphRenderers/defaults.ts |
| Navigation spine exists | navigation.ts (5 kinds); engines' "view in graph →" (ThreeDEngine:225) |
| Files graph-native but siloed | WorkspaceBrowser.tsx (FileReference containers, honest empties); no file↔graph link strips anywhere |
| zustand store present | services/store.ts:208 useOzoneStore |

---

## 8. THE 3D GRAPH-NATIVE WORLD (the operator's 2026-10-27 vision)

> "I want it like an explorable traversable 3D graph-native traversal map etc across all" — the workspace is not tabs over a substrate, it IS a navigable 3D space where every container is a node, every edge is a path, and the lenses are camera positions.

### 8.1 What the operator described
The current UI has tabs (Files/Fabric/Hierarchy/Graph/Engines) that switch between flat 2D views of graph slices. The operator wants:
- **One persistent 3D space** — containers are 3D nodes positioned by the graph's real structure (modality clusters, AMT depth, cross-link proximity)
- **Traversal as navigation** — clicking a node flies the camera to it; its cross-links expand as visible paths to neighbors; "opening" a node swaps the active lens to that modality's detail view INSIDE the 3D space (not a separate tab)
- **Dynamic lens switching** — the tab bar becomes a camera-preset selector, not a page swap; the graph persists underneath
- **All 27 modalities visible** — the discriminator fix + GraphProvider already cover this
- **Icons/indicators as 3D objects** — task pips, gate states, construction pulses rendered in-space

### 8.2 Architecture (extends GraphKit, not replaces it)
- **Layer 6 — the 3D renderer**: a `<GraphWorld3D>` component that consumes the same GraphProvider substrate. Three.js IS the right leaf renderer here (the doctrine allows it: "leaf renderers behind GraphKit's contracts"). It renders:
  - Nodes as 3D primitives (sphere/box/octahedron per modality from the shape system), colored by MODALITY_COLOR
  - Edges as 3D lines/tubes with edgeClass honesty
  - Camera as a first-person or orbital controller
  - Node selection → NodeInspector (the same primitive, rendered as a 3D-anchored overlay)
  - CrossLinkStrip → rendered as visible edges + a hover panel
- **What makes it "traversable"**: raycast picking → navigateTo (the spine) → camera fly-to → NodeInspector opens in-place. The user moves THROUGH the graph the way they currently scroll through tabs.
- **Performance**: LOD (distant nodes = dots, near = full geometry), frustum culling, and the GraphProvider's single-fetch model already bound the data.

### 8.3 What exists vs what to build
| Exists | To build |
|---|---|
| GraphProvider substrate (all 27 modalities, cross-links, files) | Three.js scene manager wired to the provider |
| MODALITY_COLOR + shape system | 3D geometry mapping per shape |
| GraphCanvas (2D SVG — stays as the mini-map/overview) | Camera controller + raycast picking + LOD |
| Navigation spine (navigateTo) | Camera fly-to integration |
| NodeCard/NodeInspector primitives | 3D-anchored overlay rendering |
| All indicator data (trace/span/task/gate) | 3D indicator meshes (pips, pulses) |

### 8.4 The tabs' role in the 3D world
The tab bar doesn't disappear — it becomes a LENS SELECTOR that sets the camera preset and filter:
- "Files" → camera flies to the file cluster, file-type nodes highlighted
- "Engines ▸ Code" → camera flies to the code cluster, code detail panel opens
- "Graph" → free orbital view of everything
- "Tasks" → task nodes (the TaskIcon data) rendered as a sidebar overlay on the 3D space
The underlying substrate NEVER remounts — switching lenses is a camera/filter change, not a page swap.

### 8.5 Dependency
three.js (or @react-three/fiber + @react-three/drei as the React wrapper) — the doctrine's leaf-renderer exception. The GraphProvider/primitives/indicators/lens contracts above are the stable API; three.js is the rendering engine behind GraphWorld3D.
