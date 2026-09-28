import type { Modality, RawGraphEdge } from "../graphViewTypes";
import type { EdgeClassification, EdgeVisual, NodeVisual } from "./types";

export const MODALITY_COLOR: Record<Modality, string> = {
  code: "#5fb3ff",
  math: "#ffb95f",
  text: "#8fe38f",
};

/** Generic fallback — exactly what C1's shell drew before any renderer existed. */
export function defaultNodeVisual(modality: Modality): NodeVisual {
  return { shape: "circle", radius: 6, fill: MODALITY_COLOR[modality] };
}

export function defaultEdgeClassification(raw: RawGraphEdge): EdgeClassification {
  return { edgeClass: raw.edge_type === "Contains" ? "structural" : "semantic", isReal: true };
}

export function defaultEdgeVisual(): EdgeVisual {
  return { stroke: "#2c3a4f", strokeWidth: 1 };
}
