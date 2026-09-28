// C6 — Text node renderer.
//
// Real (construction-verified) TextNodeType variants — the only nodes ever built
// in the single persisting path, `create_graph` in
// assets/pipelines/modalities/text/main.rs (persisted by persist_graph_container
// to graphs/text_{id}.json):
//   Document, Section, Entity, Topic, Keyword   — always built from the analysis
//   Sentence, GrammarSubject, GrammarObject     — built only when the caller
//                                                 supplies grammar `chunks`
// Every other declared variant (Paragraph, Reference, Chunk, ModalityReference,
// TrueTextSpan, FileReference, ChunkReference, SupplementarySection,
// InferredConcept) has no construction site: isReal=false, drawn in the shared
// dashed/dimmed "not real" style. Unknown strings get the same treatment.
//
// Deliberately NOT encoded visually: `provisional` (always false) and
// `hotness_score` (always the constant 0.5) — text_new_node hardcodes both, so
// they carry no per-node information today. Entity `confidence` and Topic/
// Keyword `relevance` live in `properties` (there is no top-level `confidence`
// on text nodes); C11's detail panel is the place for them.
import type { GraphViewNode, RawGraphNode } from "../graphViewTypes";
import type { ModalityNodeRenderer, NodeVisual } from "./types";
import { MODALITY_COLOR } from "./defaults";

const REAL_TYPES = new Set([
  "Document",
  "Section",
  "Entity",
  "Topic",
  "Keyword",
  "Sentence",
  "GrammarSubject",
  "GrammarObject",
]);

const BASE = MODALITY_COLOR.text;
const STROKE = "#2f7a4a";

const REAL_VISUAL: Record<string, NodeVisual> = {
  Document: { shape: "hexagon", radius: 11, fill: BASE, stroke: STROKE, glyph: "D" },
  Section: { shape: "square", radius: 7, fill: "#6fcf97", stroke: STROKE, glyph: "§" },
  Sentence: { shape: "circle", radius: 6, fill: "#c4f0c4", stroke: STROKE, glyph: "S" },
  Entity: { shape: "diamond", radius: 9, fill: "#a6e88f", stroke: STROKE, glyph: "E" },
  Topic: { shape: "triangle", radius: 9, fill: "#7fdca6", stroke: STROKE, glyph: "T" },
  Keyword: { shape: "circle", radius: 6, fill: BASE, stroke: STROKE, glyph: "K" },
  GrammarSubject: { shape: "hexagon", radius: 6, fill: "#d9f5a5", stroke: STROKE, glyph: "Su" },
  GrammarObject: { shape: "hexagon", radius: 6, fill: "#9fe0c8", stroke: STROKE, glyph: "Ob" },
};

const NOT_REAL_VISUAL: NodeVisual = {
  shape: "circle",
  radius: 6,
  fill: BASE,
  stroke: "#7a8699",
  strokeDasharray: "3 2",
  opacity: 0.45,
  glyph: "?",
};

function shorten(s: string, max = 60): string {
  return s.length > max ? `${s.slice(0, max - 1)}…` : s;
}

function labelFor(raw: RawGraphNode): string | undefined {
  const content = (raw.content ?? "").trim();
  switch (raw.node_type) {
    case "Document":
      return content ? shorten(`Document — ${content}`) : "Document";
    case "Section":
      // Section content is the real title, which is genuinely empty when the
      // analysis found none — say so rather than showing "#id".
      return content ? shorten(content) : "Section (untitled)";
    default:
      return content ? shorten(content) : undefined;
  }
}

export const textNodeRenderer: ModalityNodeRenderer = {
  modality: "text",
  classify: (raw) => ({ isReal: REAL_TYPES.has(raw.node_type), label: labelFor(raw) }),
  visual: (node: GraphViewNode) =>
    node.isReal ? REAL_VISUAL[node.nodeType] ?? NOT_REAL_VISUAL : NOT_REAL_VISUAL,
};
