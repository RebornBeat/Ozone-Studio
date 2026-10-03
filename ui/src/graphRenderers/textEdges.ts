// C7 — Text edge renderer.
//
// "Real" = a type some code path in assets/pipelines/modalities/text/main.rs
// (`create_graph`, the only path persisted to graphs/text_{id}.json) INTENDS to
// emit. Every TextGraphEdge comes from `text_new_edge`, called at:
//   * Contains — deterministic: Document->Section/Entity/Topic/Keyword/Sentence
//     and Sentence->GrammarSubject/GrammarObject (6465-6685). The only type seen
//     in any graph file on disk today (24/24 edges).
//   * grammar edges — GrammarSubject->GrammarObject inside a sentence (6698) and
//     Sentence->Sentence cross-sentence (6733). Type = an LLM-produced string
//     passed through `resolve_text_edge_type` (unrecognised => skipped, never
//     defaulted). Only built when the caller supplies grammar `chunks`. The
//     per-sentence/legacy prompts (main.rs ~3744 / ~5537) offer 20 types
//     including CausedBy; the cross-sentence prompt (~5816) offers a
//     different, smaller list that includes "Causes" as its own option —
//     until 2026-09-29 that was silently collapsed onto CausedBy by
//     `resolve_text_edge_type` (a real direction-loss bug, previously
//     flagged, now fixed: they're distinct TextEdgeType variants).
//   * SimilarTo — coreference star, anchor sentence -> each other mention (6785).
// So REAL = Contains + those 21 (20 + the now-distinct Causes). No such
// grammar edge is persisted on disk yet (data-dependent on the extractor's
// output), but any that appears was built by an intended construction site.
//
// Everything else declared in `TextEdgeType` (ContainedBy, Follows, Precedes,
// References, the Cross-Modality group, RelatesTo, RefinesTo, ForkedFrom,
// SupplementsPrompt, ContextProvides) is never prompted for or constructed:
// isReal=false, drawn in the shared dashed/dimmed "not real" style, classified by
// its declared enum group. Unknown strings get the same treatment.
//
// Class: only Contains is "structural" (deterministic scaffold). Grammar edges
// that are named PartOf/HasPart are still model-derived content claims, so they
// are "semantic", not structural. `is_cross_modal` is NOT mapped to
// "cross-modal": `text_new_edge` hardcodes it false and nothing sets it true
// (nor `cross_modal_index_id`), so no real text edge is cross-modal; that class
// is reserved for container-level SimilarTo (C8).
//
// Encoding: hue AND dash/arrow both vary so types stay distinguishable without
// colour. Not-real edges are grey, dotted, and heavily dimmed — no real family
// uses grey or that dash.
import type { EdgeClass } from "../graphViewTypes";
import type { EdgeVisual, ModalityEdgeRenderer } from "./types";

const CAUSAL: EdgeVisual = { stroke: "#f0a04b", strokeWidth: 1.6, arrow: true };
const TEMPORAL: EdgeVisual = { stroke: "#4fd1c5", strokeWidth: 1.5, strokeDasharray: "6 3", arrow: true };
const TAXONOMIC: EdgeVisual = { stroke: "#b48cff", strokeWidth: 1.5, strokeDasharray: "10 3", arrow: true };
const DERIVATION: EdgeVisual = { stroke: "#d9c46a", strokeWidth: 1.5, strokeDasharray: "3 2", arrow: true };

const REAL_VISUAL = new Map<string, EdgeVisual>([
  // Deterministic document scaffold: thin, neutral, no arrow.
  ["Contains", { stroke: "#3d4c63", strokeWidth: 1, opacity: 0.85 }],

  // Event/action causation and consequence.
  ["Performs", CAUSAL],
  ["Affects", CAUSAL],
  ["Implies", CAUSAL],
  ["CausedBy", CAUSAL],
  ["Causes", CAUSAL],
  ["Enables", CAUSAL],
  ["Prevents", CAUSAL],

  // Argumentative / discourse relations, one look each.
  ["Supports", { stroke: "#7bd88f", strokeWidth: 1.6, arrow: true }],
  ["Contradicts", { stroke: "#ff6b6b", strokeWidth: 1.8, strokeDasharray: "6 3", arrow: true }],
  ["Elaborates", { stroke: "#6fb1ff", strokeWidth: 1.4, strokeDasharray: "3 2", arrow: true }],
  ["Summarizes", { stroke: "#6fb1ff", strokeWidth: 1.4, strokeDasharray: "8 2 2 2", arrow: true }],

  ["TemporalPrecedes", TEMPORAL],
  ["TemporalFollows", TEMPORAL],

  ["PartOf", TAXONOMIC],
  ["HasPart", TAXONOMIC],
  ["InstanceOf", TAXONOMIC],
  ["HasInstance", TAXONOMIC],
  ["FunctionalRole", TAXONOMIC],

  ["DerivedFrom", DERIVATION],
  ["VersionOf", DERIVATION],

  // Symmetric-ish similarity (also the coreference star): no arrowhead.
  ["SimilarTo", { stroke: "#f78fb3", strokeWidth: 1.4 }],
]);

// Declared-only variants, classified by the TextEdgeType enum's own grouping.
const SCHEMA_ONLY_CLASS = new Map<string, EdgeClass>([
  // Structural group
  ["ContainedBy", "structural"],
  ["Follows", "structural"],
  ["Precedes", "structural"],
  // Both "Cross-modality" groups in the enum
  ["DescribesCode", "cross-modal"],
  ["DescribesImage", "cross-modal"],
  ["DescribesAudio", "cross-modal"],
  ["DescribesVideo", "cross-modal"],
  ["TranscribedFrom", "cross-modal"],
  ["ReferencesModality", "cross-modal"],
  ["ReferencedBy", "cross-modal"],
  ["DescribedBy", "cross-modal"],
  ["Describes", "cross-modal"],
  ["ImplementedIn", "cross-modal"],
  ["Implements", "cross-modal"],
  ["VisualizedAs", "cross-modal"],
  ["Visualizes", "cross-modal"],
  ["SyncedTo", "cross-modal"],
  ["SyncedBy", "cross-modal"],
  ["AnnotatedBy", "cross-modal"],
  // Semantic / versioning / supplementary
  ["References", "semantic"],
  ["RelatesTo", "semantic"],
  ["RefinesTo", "semantic"],
  ["ForkedFrom", "semantic"],
  ["SupplementsPrompt", "semantic"],
  ["ContextProvides", "semantic"],
]);

const NOT_REAL_VISUAL: EdgeVisual = {
  stroke: "#6b7a90",
  strokeWidth: 1,
  strokeDasharray: "1.5 4",
  opacity: 0.35,
};

export const textEdgeRenderer: ModalityEdgeRenderer = {
  modality: "text",
  classify: (raw) => {
    const t = raw.edge_type;
    if (t === "Contains") return { edgeClass: "structural", isReal: true };
    if (REAL_VISUAL.has(t)) return { edgeClass: "semantic", isReal: true };
    return { edgeClass: SCHEMA_ONLY_CLASS.get(t) ?? "semantic", isReal: false };
  },
  visual: (edge) => (edge.isReal ? REAL_VISUAL.get(edge.edgeType) ?? NOT_REAL_VISUAL : NOT_REAL_VISUAL),
};
