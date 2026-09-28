// C3 — Code edge renderer.
//
// Real (construction-verified) CodeEdgeType variants, per the only
// `CodeGraphEdge` construction sites in assets/pipelines/modalities/code/main.rs
// (2660/2696 Contains, 2716 Extends, 2727 Implements, 2748 Calls, 2785 Imports):
// Contains, Imports, Extends, Implements, Calls. Extends/Implements/Calls are
// same-file resolution only (bare identifiers matched against this file's own
// class/function names) — nothing here implies cross-file resolution.
//
// Every other declared variant (Exports, DependsOn, References, TypeOf,
// RelatesTo, SimilarTo, AlternativeTo, Refactors, Tests) has no construction
// site: classified by its declared semantics but isReal=false, and drawn in the
// shared "not real" style. Unknown strings get the same treatment.
import type { EdgeClass } from "../graphViewTypes";
import type { EdgeVisual, ModalityEdgeRenderer } from "./types";

const REAL_EDGE_CLASS: Record<string, EdgeClass> = {
  Contains: "structural",
  Imports: "dependency",
  Extends: "dependency",
  Implements: "dependency",
  Calls: "dependency",
};

// Declared-only variants, grouped as the CodeEdgeType enum comments group them.
const SCHEMA_ONLY_EDGE_CLASS: Record<string, EdgeClass> = {
  Exports: "structural",
  DependsOn: "dependency",
  References: "dependency",
  TypeOf: "dependency",
  RelatesTo: "semantic",
  SimilarTo: "semantic",
  AlternativeTo: "semantic",
  Refactors: "semantic",
  Tests: "semantic",
};

const REAL_VISUAL: Record<string, EdgeVisual> = {
  Contains: { stroke: "#3d4c63", strokeWidth: 1 },
  Imports: { stroke: "#4fd1c5", strokeWidth: 1.4, arrow: true },
  Extends: { stroke: "#c792ea", strokeWidth: 2, arrow: true },
  Implements: { stroke: "#f78fb3", strokeWidth: 1.8, arrow: true },
  Calls: { stroke: "#ff8a65", strokeWidth: 1.4, strokeDasharray: "6 3", arrow: true },
};

const NOT_REAL_VISUAL: EdgeVisual = {
  stroke: "#7a8699",
  strokeWidth: 1,
  strokeDasharray: "1.5 3",
  opacity: 0.4,
};

export const codeEdgeRenderer: ModalityEdgeRenderer = {
  modality: "code",
  classify: (raw) => {
    const real = REAL_EDGE_CLASS[raw.edge_type];
    if (real) return { edgeClass: real, isReal: true };
    return { edgeClass: SCHEMA_ONLY_EDGE_CLASS[raw.edge_type] ?? "semantic", isReal: false };
  },
  visual: (edge) => (edge.isReal ? REAL_VISUAL[edge.edgeType] ?? NOT_REAL_VISUAL : NOT_REAL_VISUAL),
};
