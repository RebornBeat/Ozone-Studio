/**
 * Graph View shared contract (Batch C, fork C1) — the types every C2-C13
 * per-modality/edge-class renderer implements against.
 *
 * Two structurally DIFFERENT real backend sources feed this view, and this
 * file keeps them distinct rather than pretending they're one thing:
 *
 * 1. Per-modality persisted graph JSON (`RawModalityGraph`) — the real
 *    content sitting behind a `ModalityGraph` container's `object_store_path`
 *    (B1), fetched via `GetContainerContent` (B0). Each modality pipeline
 *    (code/math/text) defines its OWN `node_type`/`edge_type` enum and
 *    serializes it as a plain string — confirmed directly against
 *    `assets/pipelines/modalities/{code,math,text}/main.rs` and a real
 *    on-disk graph file (`assets/pipelines/modalities/math/zsei_data/graphs/
 *    math_124277799.json`). These enums are NOT the same as `RelationType`
 *    below, and node/edge shapes differ per modality (e.g. code nodes carry
 *    `name`+`position`, math/text nodes carry `content`+`label`/`content`).
 *
 * 2. Container-level relationships (`ContainerRelation`) — any container's
 *    `local_state.context.relationships: Relation[]` (B16, always present,
 *    no separate endpoint), using the generic `RelationType` enum
 *    (`src/types/container.rs:267-325`). This is the mechanism behind
 *    cross-modal `SimilarTo` (C8), AMT `ForkOf`/`ContinuedBy` lineage (C9),
 *    and jurisdiction `RelatedTo` (C10) — genuinely different containers and
 *    a genuinely different fetch path than #1.
 *
 * Renderers (C2-C13) convert from these two raw shapes into the unified
 * `GraphViewNode`/`GraphViewEdge` presentation types the canvas (C1) draws.
 * Doctrine: `raw` is always the untouched backend payload, `isReal` is only
 * ever set from a genuine registry/taxonomy check (see
 * docs/GRAPH_RELATIONSHIP_REGISTRY.md) — never fabricated, never guessed.
 */

export type Modality = "code" | "math" | "text";

// ─────────────────────────────────────────────────────────────────────────
// Raw modality graph JSON — exactly as persisted at object_store_path.
// Field presence genuinely differs per modality; treat absent fields as
// absent, not as zero/empty placeholders.
// ─────────────────────────────────────────────────────────────────────────

export interface RawGraphNode {
  node_id: number;
  /** Real per-modality enum value serialized as a string, e.g. "ProofStep"
   * (math), "File" (code), "Entity" (text). See docs/GRAPH_RELATIONSHIP_REGISTRY.md
   * for which variants are REAL (backed by real construction sites) vs.
   * schema-only (declared but never constructed). */
  node_type: string;
  /** Math nodes carry `label` (short) + `content` (fuller text). */
  label?: string;
  /** Code nodes carry `name` instead of `label`. */
  name?: string;
  /** Math/text nodes carry `content`; code nodes do not. */
  content?: string;
  step_number?: number | null;
  confidence?: number;
  properties: Record<string, unknown>;
  /** Code: `CodePosition`. Text: `TextPosition`. Math nodes have no position. */
  position?: unknown;
  annotations?: unknown[];
  /** Text nodes only (TextGraphNode's "UNIVERSAL NODE FIELDS") — real, used
   * by C11's detail panel when present. */
  materialized_path?: string;
  keywords?: string[];
  provisional?: boolean;
  hotness_score?: number;
  // Real schemas carry more modality-specific fields than are enumerated
  // here (see the source structs cited above) — this is deliberately not
  // exhaustive; renderers should read `properties` for anything not
  // promoted to a named field here.
  [extra: string]: unknown;
}

export interface RawGraphEdge {
  /** Present on math ("MathGraphEdge") and text ("TextGraphEdge") edges.
   * Code's `CodeGraphEdge` (assets/pipelines/modalities/code/main.rs:1042)
   * has NO `edge_id` field at all — confirmed by direct source read.
   * Renderers must synthesize a stable id (e.g. `${from_node}-${edge_type}-${to_node}`)
   * when this is absent rather than assuming it's always present. */
  edge_id?: number;
  from_node: number;
  to_node: number;
  /** Real per-modality edge-type enum as a string. Distinct enum per
   * modality — e.g. math's real "Uses" is `MathEdgeType::Uses`, a totally
   * different type from text's `TextEdgeType::References`, even though
   * both might render with similar visual weight. */
  edge_type: string;
  weight: number;
  properties: Record<string, unknown>;
  /** Text edges only (TextGraphEdge's "UNIVERSAL EDGE FIELDS"). */
  is_cross_modal?: boolean;
  [extra: string]: unknown;
}

/** The real wrapper shape found on disk (verified against a live math graph
 * file, NOT guessed): `{content_type, created_at, edges, graph_id, metadata,
 * modality, name, nodes}`. Note the real field is `metadata`, not `analysis`
 * — earlier planning language guessed `analysis`; corrected here against
 * the actual file. `name`/`content_type`/`created_at` were only confirmed
 * present on the math sample; treat them as optional until code/text
 * samples are checked by a later fork. */
export interface RawModalityGraph {
  graph_id: number;
  /** Absent on the older code wrapper (verified on all 5 on-disk code graphs:
   * keys are exactly {analysis, edges, graph_id, nodes}). Math/text carry it. */
  modality?: string;
  name?: string;
  content_type?: string;
  created_at?: string;
  nodes: RawGraphNode[];
  edges: RawGraphEdge[];
  /** Math + text wrappers. Code's wrapper uses `analysis` instead. */
  metadata?: Record<string, unknown>;
  /** Code wrapper only (verified against on-disk code graphs). */
  analysis?: Record<string, unknown>;
}

// ─────────────────────────────────────────────────────────────────────────
// Container-level relationships — Context.relationships, a SEPARATE
// mechanism from the raw graph JSON above. Real fields confirmed against
// `src/types/container.rs:246-262`.
// ─────────────────────────────────────────────────────────────────────────

export interface ContainerRelation {
  target_id: number;
  /** Real `RelationType` variant name, e.g. "SimilarTo" | "ForkOf" |
   * "ContinuedBy" | "RelatedTo" | "Contains" | "ImportsFrom" | "CallsTo" |
   * "Implements" | "Extends" | ... — full list in container.rs:267-325.
   * `JurisdictionScope` (=70) is declared but NOT wired to any real
   * construction site as of 2026-09-22 per that enum's own doc comment —
   * do not render it as if it were live. */
  relation_type: string;
  /** Optional: legacy keyword-derived lineage (`amt-fork-of:<id>`) has no recorded confidence — never invent one. */
  confidence?: number;
  /** Real `DiscoveryMethod` variant: "Manual" | "ZeroShot" | "Traversal" |
   * "MLPrediction" | "CodeAnalysis" | "TextAnalysis" | "WebNavigation".
   * NOTE: there is no "MathAnalysis" variant in the real enum (confirmed
   * against src/types/container.rs:328-337) — math-sourced relations use
   * one of the above, most likely "Traversal" or "MLPrediction"; do not
   * invent a "MathAnalysis" label anywhere in the UI. */
  discovered_via?: string;
  /** Present only when the edge came from the relationship-graph proximity
   * walk, not keyword matching — real field, was silently dropped by serde
   * until this session's fix. */
  graph_hops?: number;
}

// ─────────────────────────────────────────────────────────────────────────
// Canvas-level unified presentation types — what C2-C13 renderers produce
// and what the C1 canvas shell (this fork) actually draws.
// ─────────────────────────────────────────────────────────────────────────

export type EdgeClass =
  | "structural"   // Contains, PartOf, DependsOn — same-graph structure
  | "dependency"   // Imports/Calls/Extends/Implements-class code edges
  | "semantic"     // content-derived (Uses, References, SimilarTo w/in a graph)
  | "cross-modal"  // SimilarTo linking different modality graphs (C8)
  | "lineage"      // AMT ForkOf/ContinuedBy (C9)
  | "governance";  // Jurisdiction RelatedTo (C10)

/** A container drawn as a node (C8-C10 overlays): container-to-container
 * relations (SimilarTo, ForkOf/ContinuedBy, RelatedTo) connect containers,
 * not in-graph nodes, so each participating real container becomes one "hub"
 * node. Hub nodes always come from real container data, never fabricated. */
export type HubKind = "modality-graph" | "amt" | "jurisdiction";

export interface GraphViewNode {
  /** Unique across the whole multi-modality canvas. Hub nodes use `hub:{containerId}`. */
  id: string;
  /** "container" for hub nodes (see `hub`), otherwise the graph's modality. */
  modality: Modality | "container";
  /** Present only on hub nodes. */
  hub?: { kind: HubKind; containerId: number; title: string };
  /** Real backend node_type string, unmodified — never remapped to a
   * "friendlier" label that would obscure what's actually real. */
  nodeType: string;
  /** Renderer-chosen human label, derived from content/name/label per
   * modality — for canvas display only, `raw` holds the real payload. */
  label: string;
  contentPreview?: string;
  confidence?: number;
  /** True only when nodeType is a REAL, construction-verified type per
   * docs/GRAPH_RELATIONSHIP_REGISTRY.md. A renderer MUST set this honestly
   * — false means "schema-only, no real data exists for this type yet",
   * and the canvas should visually mark it (per this doc's doctrine: never
   * fabricate data, always show an honest state) rather than hide it. */
  isReal: boolean;
  raw: RawGraphNode;
  /** The real ModalityGraph container id this node's graph came from —
   * provenance for the detail panel (C11) and for re-fetching. */
  sourceContainerId: number;
}

export interface GraphViewEdge {
  id: string;
  from: string; // GraphViewNode.id
  to: string;   // GraphViewNode.id
  /** Real backend edge_type / relation_type string, unmodified. */
  edgeType: string;
  edgeClass: EdgeClass;
  confidence?: number;
  discoveredVia?: string;
  isReal: boolean;
  raw: RawGraphEdge | ContainerRelation;
}

export interface GraphViewData {
  nodes: GraphViewNode[];
  edges: GraphViewEdge[];
  /** Real ModalityGraph container ids this data was assembled from —
   * surfaced so any consumer can cite its real source, per this plan's
   * doctrine ("every fork should cite the real file/data source it renders"). */
  sourceContainers: number[];
  /** Real `local_state.context.relationships` for each fetched
   * ModalityGraph container, keyed by container id — passed through as-is
   * (already fetched for free alongside each container's metadata, B16
   * needs no separate call). C1 does not render these as canvas edges;
   * they're container-to-container (SimilarTo/ForkOf/RelatedTo), a
   * different node space than the in-graph nodes above. C8 (cross-modal
   * SimilarTo), C9 (AMT ForkOf/ContinuedBy lineage), and C10 (jurisdiction
   * RelatedTo) are the intended consumers of this field. */
  containerRelations: Record<number, ContainerRelation[]>;
}

/** Loading/error/empty states a fetch hook must report honestly — no
 * fabricated fallback content on any branch. */
export type GraphViewStatus =
  | { kind: "loading" }
  | { kind: "error"; message: string }
  | { kind: "empty" } // real fetch succeeded, genuinely zero graph data exists yet
  | { kind: "ready"; data: GraphViewData };
