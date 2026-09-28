// C5 — Math edge renderer.
//
// "Real" = the type is pushed by build_math_graph in
// assets/pipelines/modalities/math/main.rs (MathEdgeType construction sites,
// ~lines 2431-2573). The other MathEdgeType variants (Derives, Requires,
// Implies, References, BindsVariable, UsesVariable, Generalizes, Specializes,
// Contradicts, SimilarTo, ImplementedBy, RepresentedBy) are declared but never
// constructed, so they are reported isReal=false and drawn dashed/dimmed.
import type { RawGraphEdge } from "../graphViewTypes";
import type { EdgeClassification, EdgeVisual, ModalityEdgeRenderer } from "./types";

type RealMathEdge = "Contains" | "FollowsStep" | "Uses" | "Defines" | "AssumesIn" | "DischargesIn";

const REAL_EDGE_TYPES: ReadonlySet<string> = new Set<RealMathEdge>([
  "Contains",
  "FollowsStep",
  "Uses",
  "Defines",
  "AssumesIn",
  "DischargesIn",
]);

function classify(raw: RawGraphEdge): EdgeClassification {
  const isReal = REAL_EDGE_TYPES.has(raw.edge_type);
  // Contains/FollowsStep are unconditional structure. Uses/Defines/AssumesIn/
  // DischargesIn are derived from the analysed content of each proof step
  // (citations, introduced variables, assumptions), so they can be absent.
  const structural = raw.edge_type === "Contains" || raw.edge_type === "FollowsStep";
  return { edgeClass: structural ? "structural" : "semantic", isReal };
}

// Colour is never the only channel: every type also differs by dash pattern
// and/or arrowhead so they stay distinguishable without hue.
const VISUALS: Record<RealMathEdge, EdgeVisual> = {
  // Presentation hierarchy: thin, neutral, no arrow — background structure.
  Contains: { stroke: "#3a4a63", strokeWidth: 1, opacity: 0.85 },
  // Always-true presentation order: solid + arrowed (step i → step i-1).
  FollowsStep: { stroke: "#8fa3c0", strokeWidth: 1.5, arrow: true },
  // Real cited dependency: dashed amber (math's node hue) + arrowed.
  Uses: { stroke: "#ffb95f", strokeWidth: 1.8, strokeDasharray: "6 3", arrow: true },
  // Step introduces a variable: dotted violet + arrowed.
  Defines: { stroke: "#b48cff", strokeWidth: 1.5, strokeDasharray: "1.5 3", arrow: true },
  // Step makes / discharges an assumption: teal vs pink, different dashes.
  AssumesIn: { stroke: "#4fd1c5", strokeWidth: 1.5, strokeDasharray: "3 2", arrow: true },
  DischargesIn: { stroke: "#ff8fa3", strokeWidth: 1.5, strokeDasharray: "8 2 2 2", arrow: true },
};

// Declared-but-unconstructed (or unknown) type: visibly "not real", never
// styled like a confirmed type.
const NOT_REAL_VISUAL: EdgeVisual = {
  stroke: "#6b7a90",
  strokeWidth: 1,
  strokeDasharray: "2 4",
  opacity: 0.5,
};

export const mathEdgeRenderer: ModalityEdgeRenderer = {
  modality: "math",
  classify,
  visual: (edge) =>
    edge.isReal && REAL_EDGE_TYPES.has(edge.edgeType)
      ? VISUALS[edge.edgeType as RealMathEdge]
      : NOT_REAL_VISUAL,
};
