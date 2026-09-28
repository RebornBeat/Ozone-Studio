/**
 * Renderer plugin contract for the Graph View (Batch C).
 *
 * Each modality/edge-class fork owns exactly ONE file under graphRenderers/
 * and implements one of these interfaces. GraphView.tsx and graphViewData.ts
 * only ever call the dispatch functions in ./index.ts — forks never edit them.
 *
 * Doctrine: classify() decides `isReal` from a genuine registry check
 * (docs/GRAPH_RELATIONSHIP_REGISTRY.md + the pipeline source that constructs
 * the type), never a guess. visual() must render a not-real/unknown type in a
 * visibly distinct (e.g. dashed, dimmed) way, never hide it and never style it
 * as if it were a confirmed real type.
 */
import type {
  EdgeClass,
  GraphViewEdge,
  GraphViewNode,
  Modality,
  RawGraphEdge,
  RawGraphNode,
} from "../graphViewTypes";

export type NodeShape = "circle" | "square" | "diamond" | "hexagon" | "triangle";

export interface NodeVisual {
  shape: NodeShape;
  radius: number;
  fill: string;
  stroke?: string;
  strokeWidth?: number;
  /** e.g. "3 2" — the honest visual marker for a not-real / unknown type. */
  strokeDasharray?: string;
  opacity?: number;
  /** Short glyph drawn inside the shape (1-2 chars), e.g. "ƒ". */
  glyph?: string;
}

export interface EdgeVisual {
  stroke: string;
  strokeWidth: number;
  strokeDasharray?: string;
  opacity?: number;
  /** Draw an arrowhead at the `to` end (directed edges). */
  arrow?: boolean;
}

export interface NodeClassification {
  isReal: boolean;
  /** Optional better label than the generic label/name/content fallback. */
  label?: string;
  contentPreview?: string;
}

export interface EdgeClassification {
  edgeClass: EdgeClass;
  isReal: boolean;
}

export interface ModalityNodeRenderer {
  modality: Modality;
  classify(raw: RawGraphNode): NodeClassification;
  visual(node: GraphViewNode): NodeVisual;
}

export interface ModalityEdgeRenderer {
  modality: Modality;
  classify(raw: RawGraphEdge): EdgeClassification;
  visual(edge: GraphViewEdge): EdgeVisual;
}
