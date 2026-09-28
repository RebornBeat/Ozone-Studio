// C4 — Math node renderer.
//
// Source of truth: `MathGraphNodeType` (assets/pipelines/modalities/math/main.rs:1236),
// the PERSISTED graph node enum — not `MathNodeType` (main.rs:607), which is the
// expression parse-tree enum (Number/BinaryOp/...) and never appears in a saved graph.
//
// Construction-verified (`MathGraphNode { node_type: MathGraphNodeType::X, .. }` sites,
// `create_graph`): Root (2396), ProofStep (2416), Variable (2504 proof-side from
// `step.introduced_variables`, 2557 ParseExpression-side), Assumption (2478, from
// `step.assumptions`). Root/ProofStep/Variable are seen in every persisted graph on
// disk. Assumption has a real constructor but is gated on data that `analyze_proof`
// still hardcodes empty (2142-2144; `extract_variable_introduction`/
// `extract_assumption_introduction` at 1683/1702 are defined but never called), so it
// only appears if a caller supplies structured steps with assumptions — treated as
// real because a node of this type can only exist if that constructor ran.
//
// Declared with NO constructor anywhere: Expression, Axiom, Theorem, Definition,
// Constant, Scope. They (and any unknown string) are isReal=false and drawn in the
// dashed/dimmed "schema-only" style — never styled like a confirmed type.
import type { GraphViewNode } from "../graphViewTypes";
import type { ModalityNodeRenderer, NodeVisual } from "./types";
import { MODALITY_COLOR } from "./defaults";

const MATH = MODALITY_COLOR.math;

const REAL_NODE_TYPES = new Set(["Root", "ProofStep", "Variable", "Assumption"]);

function proofStepGlyph(node: GraphViewNode): string {
  const n = node.raw.step_number;
  if (typeof n === "number" && n >= 0 && n <= 99) return String(n);
  return "¶";
}

export const mathNodeRenderer: ModalityNodeRenderer = {
  modality: "math",

  classify: (raw) => ({
    isReal: REAL_NODE_TYPES.has(raw.node_type),
    label: typeof raw.label === "string" && raw.label.length > 0 ? raw.label : undefined,
    contentPreview: typeof raw.content === "string" && raw.content.length > 0 ? raw.content : undefined,
  }),

  visual: (node): NodeVisual => {
    if (!node.isReal) {
      return {
        shape: "circle",
        radius: 6,
        fill: MATH,
        stroke: MATH,
        strokeWidth: 1.5,
        strokeDasharray: "3 2",
        opacity: 0.35,
      };
    }
    switch (node.nodeType) {
      case "Root":
        return { shape: "hexagon", radius: 12, fill: MATH, glyph: "Σ" };
      case "ProofStep":
        return { shape: "square", radius: 9, fill: MATH, glyph: proofStepGlyph(node) };
      case "Variable":
        return { shape: "diamond", radius: 9, fill: "#ffd699", glyph: "x" };
      case "Assumption":
        return { shape: "triangle", radius: 9, fill: "#e59a3d", glyph: "A" };
      default:
        // Unreachable while REAL_NODE_TYPES and this switch stay in sync.
        return { shape: "circle", radius: 6, fill: MATH, opacity: 0.35, strokeDasharray: "3 2", stroke: MATH };
    }
  },
};
