/**
 * Container-level overlays (C8 cross-modal, C9 lineage, C10 governance).
 *
 * These relations connect CONTAINERS, so each overlay loads its own real data
 * and returns hub nodes + edges that GraphView merges into the canvas when the
 * user enables the overlay. Each overlay lives in its own file under
 * ./overlays/ and is owned by exactly one fork; this registry is scaffold.
 */
import type { EdgeClass, GraphViewData, GraphViewEdge, GraphViewNode } from "../graphViewTypes";
import type { EdgeVisual, NodeVisual } from "./types";
import { crossModalOverlay } from "./overlays/crossModal";
import { lineageOverlay } from "./overlays/lineage";
import { governanceOverlay } from "./overlays/governance";

export interface OverlayContext {
  projectId: number;
  /** The already-loaded modality graph data (incl. per-container relations). */
  graphData: GraphViewData;
}
export interface OverlayResult {
  nodes: GraphViewNode[];
  edges: GraphViewEdge[];
}
export interface GraphOverlay {
  id: "cross-modal" | "lineage" | "governance";
  label: string;
  /** One line shown next to the toggle. */
  description: string;
  edgeClass: EdgeClass;
  /** Real data only; an empty result is honest, throwing surfaces a real error. */
  load(ctx: OverlayContext): Promise<OverlayResult>;
  nodeVisual(node: GraphViewNode): NodeVisual;
  edgeVisual(edge: GraphViewEdge): EdgeVisual;
}

export const GRAPH_OVERLAYS: GraphOverlay[] = [crossModalOverlay, lineageOverlay, governanceOverlay];

export function overlayForEdge(edge: GraphViewEdge): GraphOverlay | undefined {
  if (!("target_id" in (edge.raw as object))) return undefined; // in-graph edge
  return GRAPH_OVERLAYS.find((o) => o.edgeClass === edge.edgeClass);
}
export function overlayForNode(node: GraphViewNode): GraphOverlay | undefined {
  if (!node.hub) return undefined;
  const byKind = { "modality-graph": "cross-modal", amt: "lineage", jurisdiction: "governance" } as const;
  return GRAPH_OVERLAYS.find((o) => o.id === byKind[node.hub!.kind]);
}
