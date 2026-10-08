import type { Modality, RawGraphEdge } from "../graphViewTypes";
import type { EdgeClassification, EdgeVisual, NodeClassification, NodeVisual } from "./types";

// Original 4 colors unchanged. The other 23 (added 2026-10-07, see
// graphViewTypes.ts) are grouped by rough category so related modalities
// read as a visual family at a glance; exact hues are a display choice, not
// backend-derived.
export const MODALITY_COLOR: Record<Modality, string> = {
  code: "#5fb3ff",
  math: "#ffb95f",
  text: "#8fe38f",
  image: "#e585d8",
  // spatial / geometric
  "3D": "#4fc3f7",
  depth: "#29b6f6",
  geospatial: "#0288d1",
  kinematics: "#0097a7",
  CAD: "#00acc1",
  // life sciences
  biology: "#66bb6a",
  dna: "#2e7d32",
  chemistry: "#9ccc65",
  proteomics: "#558b2f",
  // neuro / bio-signal
  BCI: "#ab47bc",
  eeg: "#8e24aa",
  // audio / haptic
  audio: "#ffa726",
  sound: "#fb8c00",
  haptic: "#ef6c00",
  // sensing / EM
  radar: "#ef5350",
  sonar: "#e53935",
  electromagnetic: "#d81b60",
  hyperspectral: "#c2185b",
  thermal: "#f06292",
  // systems / misc
  control: "#78909c",
  network: "#546e7a",
  IMU: "#8d6e63",
  video: "#7e57c2",
};

/** Generic fallback — exactly what C1's shell drew before any renderer existed. */
export function defaultNodeVisual(modality: Modality): NodeVisual {
  return { shape: "circle", radius: 6, fill: MODALITY_COLOR[modality] };
}

export function defaultEdgeClassification(raw: RawGraphEdge): EdgeClassification {
  return { edgeClass: raw.edge_type === "Contains" ? "structural" : "semantic", isReal: true };
}

/** Generic fallback for a modality with no dedicated classify() yet — real,
 * no special label/preview override (falls back to the node's own
 * label/name/content elsewhere, per NodeClassification's own contract). */
export function defaultNodeClassification(): NodeClassification {
  return { isReal: true };
}

export function defaultEdgeVisual(): EdgeVisual {
  return { stroke: "#2c3a4f", strokeWidth: 1 };
}
