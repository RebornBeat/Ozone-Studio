// Image edge renderer.
//
// Real (construction-verified) ImageEdgeType variants, per the only
// `ImageGraphEdge` construction sites in assets/pipelines/modalities/image/
// main.rs (`build_image_graph`, ~1405-1649): Contains (every Object/Region/
// Text/Face/Color node gets one from the root Image node), and the pairwise
// spatial set computed between Object nodes only — Overlaps (bounding-box
// intersection, weight = real IoU), else exactly one of LeftOf/RightOf/
// Above/Below (nearest-axis relative position, weight fixed at 1.0). This is
// also what YOLO detection expands (`yolo_graph`, tools/visual-mcp) — a real
// detected-object pair's bounding boxes feed this exact same spatial logic,
// e.g. the verified `person Above person` / `Overlaps` edges on container
// 40277.
//
// Every other declared variant (ContainedBy, AdjacentTo, SimilarTo,
// RelatesTo, PartOf, DescribedBy, Describes, ImplementedBy, Represents) has
// no construction site: classified by declared semantics but isReal=false.
import type { EdgeClass } from "../graphViewTypes";
import type { EdgeVisual, ModalityEdgeRenderer } from "./types";

const REAL_EDGE_CLASS: Record<string, EdgeClass> = {
  Contains: "structural",
  Overlaps: "semantic",
  LeftOf: "semantic",
  RightOf: "semantic",
  Above: "semantic",
  Below: "semantic",
};

const SCHEMA_ONLY_EDGE_CLASS: Record<string, EdgeClass> = {
  ContainedBy: "structural",
  AdjacentTo: "semantic",
  SimilarTo: "semantic",
  RelatesTo: "semantic",
  PartOf: "structural",
  DescribedBy: "cross-modal",
  Describes: "cross-modal",
  ImplementedBy: "cross-modal",
  Represents: "cross-modal",
};

const REAL_VISUAL: Record<string, EdgeVisual> = {
  Contains: { stroke: "#3d4c63", strokeWidth: 1 },
  Overlaps: { stroke: "#e585d8", strokeWidth: 1.8 },
  LeftOf: { stroke: "#e5a5d8", strokeWidth: 1, strokeDasharray: "4 2", arrow: true },
  RightOf: { stroke: "#e5a5d8", strokeWidth: 1, strokeDasharray: "4 2", arrow: true },
  Above: { stroke: "#e5a5d8", strokeWidth: 1, strokeDasharray: "4 2", arrow: true },
  Below: { stroke: "#e5a5d8", strokeWidth: 1, strokeDasharray: "4 2", arrow: true },
};

const NOT_REAL_VISUAL: EdgeVisual = {
  stroke: "#7a8699",
  strokeWidth: 1,
  strokeDasharray: "1.5 3",
  opacity: 0.4,
};

export const imageEdgeRenderer: ModalityEdgeRenderer = {
  modality: "image",
  classify: (raw) => {
    const real = REAL_EDGE_CLASS[raw.edge_type];
    if (real) return { edgeClass: real, isReal: true };
    return { edgeClass: SCHEMA_ONLY_EDGE_CLASS[raw.edge_type] ?? "semantic", isReal: false };
  },
  visual: (edge) => (edge.isReal ? REAL_VISUAL[edge.edgeType] ?? NOT_REAL_VISUAL : NOT_REAL_VISUAL),
};
