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
import { defaultEdgeClassification, defaultEdgeVisual, defaultNodeClassification, defaultNodeVisual } from "./defaults";
import { codeNodeRenderer } from "./codeNodes";
import { codeEdgeRenderer } from "./codeEdges";
import { mathNodeRenderer } from "./mathNodes";
import { mathEdgeRenderer } from "./mathEdges";
import { textNodeRenderer } from "./textNodes";
import { textEdgeRenderer } from "./textEdges";
import { imageNodeRenderer } from "./imageNodes";
import { imageEdgeRenderer } from "./imageEdges";
import { overlayForEdge, overlayForNode } from "./overlays";

// Only 4 modalities have a dedicated, hand-written renderer. The other 23
// (graphViewTypes.ts, added 2026-10-07) fall through to the generic default
// below — real data, generic presentation, never a crash. A dedicated
// renderer file is still the right place to add real per-modality logic
// later; this is not a substitute for that, only the safe absence of it.
const NODE_RENDERERS: Partial<Record<Modality, ModalityNodeRenderer>> = {
  code: codeNodeRenderer,
  math: mathNodeRenderer,
  text: textNodeRenderer,
  image: imageNodeRenderer,
};
const EDGE_RENDERERS: Partial<Record<Modality, ModalityEdgeRenderer>> = {
  code: codeEdgeRenderer,
  math: mathEdgeRenderer,
  text: textEdgeRenderer,
  image: imageEdgeRenderer,
};

export function classifyNode(modality: Modality, raw: RawGraphNode): NodeClassification {
  return NODE_RENDERERS[modality]?.classify(raw) ?? defaultNodeClassification();
}
export function classifyEdge(modality: Modality, raw: RawGraphEdge): EdgeClassification {
  return EDGE_RENDERERS[modality]?.classify(raw) ?? defaultEdgeClassification(raw);
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
  return EDGE_RENDERERS[modality]?.visual(edge) ?? defaultEdgeVisual();
}
export { defaultEdgeClassification };
