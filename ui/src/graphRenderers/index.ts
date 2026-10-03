/**
 * Dispatch layer — the ONLY thing GraphView.tsx / graphViewData.ts call.
 * Per-modality logic lives in the six renderer files; forks never edit this.
 */
import type { GraphViewEdge, GraphViewNode, Modality, RawGraphEdge, RawGraphNode } from "../graphViewTypes";
import type {
  EdgeClassification,
  EdgeVisual,
  ModalityEdgeRenderer,
  ModalityNodeRenderer,
  NodeClassification,
  NodeVisual,
} from "./types";
import { defaultEdgeClassification, defaultEdgeVisual, defaultNodeVisual } from "./defaults";
import { codeNodeRenderer } from "./codeNodes";
import { codeEdgeRenderer } from "./codeEdges";
import { mathNodeRenderer } from "./mathNodes";
import { mathEdgeRenderer } from "./mathEdges";
import { textNodeRenderer } from "./textNodes";
import { textEdgeRenderer } from "./textEdges";
import { imageNodeRenderer } from "./imageNodes";
import { imageEdgeRenderer } from "./imageEdges";
import { overlayForEdge, overlayForNode } from "./overlays";

const NODE_RENDERERS: Record<Modality, ModalityNodeRenderer> = {
  code: codeNodeRenderer,
  math: mathNodeRenderer,
  text: textNodeRenderer,
  image: imageNodeRenderer,
};
const EDGE_RENDERERS: Record<Modality, ModalityEdgeRenderer> = {
  code: codeEdgeRenderer,
  math: mathEdgeRenderer,
  text: textEdgeRenderer,
  image: imageEdgeRenderer,
};

export function classifyNode(modality: Modality, raw: RawGraphNode): NodeClassification {
  return NODE_RENDERERS[modality].classify(raw);
}
export function classifyEdge(modality: Modality, raw: RawGraphEdge): EdgeClassification {
  return EDGE_RENDERERS[modality].classify(raw);
}
const HUB_FALLBACK: NodeVisual = { shape: "square", radius: 9, fill: "#9aa5b5", opacity: 0.6, strokeDasharray: "3 2" };

export function nodeVisual(node: GraphViewNode): NodeVisual {
  if (node.hub || node.modality === "container") {
    return overlayForNode(node)?.nodeVisual(node) ?? HUB_FALLBACK;
  }
  return NODE_RENDERERS[node.modality]?.visual(node) ?? defaultNodeVisual(node.modality);
}
/** Container-level edges (raw is a ContainerRelation) are drawn by the
 * overlay that owns their edgeClass (C8-C10); in-graph edges by the
 * per-modality renderer. */
export function edgeVisual(edge: GraphViewEdge, modality: Modality | "container" | null): EdgeVisual {
  const overlay = overlayForEdge(edge);
  if (overlay) return overlay.edgeVisual(edge);
  if (!modality || modality === "container") return defaultEdgeVisual();
  return EDGE_RENDERERS[modality].visual(edge);
}
export { defaultEdgeClassification };
