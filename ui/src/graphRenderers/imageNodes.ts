/**
 * Image modality node renderer.
 *
 * Ground truth (assets/pipelines/modalities/image/main.rs, `build_image_graph`
 * around line 1405+): `ImageNodeType` declares 8 variants but only 6 are ever
 * CONSTRUCTED — Image (1417), Object (1448), Region (1475), Text (1504), Face
 * (1536), Color (1566). Composition and Quality are schema-only (declared,
 * never built). Any node whose type is not one of the 6 is classified
 * `isReal: false` and drawn dashed/dimmed, per this plugin family's shared
 * doctrine — never hidden, never styled as if confirmed real.
 *
 * Every constructed node carries a real `bounding_box` (except Color, which
 * has none — a dominant-color entry has no spatial location) and a real
 * `confidence`. This is also the modality YOLO detection (`yolo_graph`,
 * tools/visual-mcp) expands: a real detection becomes a real Object node with
 * `label` = the detected class (e.g. "person") and `properties` = whatever
 * attributes the detector attached — rendered generically below since
 * detector-specific attribute keys aren't part of the pipeline's own fixed
 * schema.
 */
import type { GraphViewNode, RawGraphNode } from "../graphViewTypes";
import type { ModalityNodeRenderer, NodeClassification, NodeVisual } from "./types";
import { MODALITY_COLOR } from "./defaults";

const REAL_NODE_TYPES: readonly string[] = ["Image", "Object", "Region", "Text", "Face", "Color"];

function isRealNodeType(nodeType: string): boolean {
  return REAL_NODE_TYPES.includes(nodeType);
}

function prop(raw: RawGraphNode, key: string): unknown {
  const props = raw.properties;
  return props && typeof props === "object" ? props[key] : undefined;
}
function propStr(raw: RawGraphNode, key: string): string | undefined {
  const v = prop(raw, key);
  return typeof v === "string" && v.length > 0 ? v : undefined;
}
function propNum(raw: RawGraphNode, key: string): number | undefined {
  const v = prop(raw, key);
  return typeof v === "number" && Number.isFinite(v) ? v : undefined;
}

function confidencePct(raw: RawGraphNode): string | undefined {
  return typeof raw.confidence === "number" ? `${(raw.confidence * 100).toFixed(0)}%` : undefined;
}

function fallbackLabel(raw: RawGraphNode): string {
  return `${raw.node_type} #${raw.node_id}`;
}

// ── classify ───────────────────────────────────────────────────────────────

function classify(raw: RawGraphNode): NodeClassification {
  const isReal = isRealNodeType(raw.node_type);
  const parts: string[] = [];

  switch (raw.node_type) {
    case "Image": {
      const w = propNum(raw, "width");
      const h = propNum(raw, "height");
      if (w !== undefined && h !== undefined) parts.push(`${w}x${h}`);
      const fmt = propStr(raw, "format");
      if (fmt) parts.push(fmt);
      const cs = propStr(raw, "color_space");
      if (cs) parts.push(cs);
      return { isReal, label: "Image", contentPreview: parts.length > 0 ? parts.join(" · ") : undefined };
    }
    case "Object": {
      // label is the real detected class (pipeline-native or YOLO/registry-
      // extended, e.g. "person") — never relabeled here.
      const label = raw.label || fallbackLabel(raw);
      const conf = confidencePct(raw);
      if (conf) parts.push(`conf ${conf}`);
      // Detector attributes are arbitrary per-detector — surface real keys
      // present rather than guessing a fixed set.
      const attrEntries = Object.entries(raw.properties ?? {}).filter(([k]) => k !== "label");
      if (attrEntries.length > 0) {
        const kv = attrEntries.map(([k, v]) => `${k}: ${JSON.stringify(v)}`).join(", ");
        parts.push(kv.length > 100 ? `${kv.slice(0, 97)}...` : kv);
      }
      return { isReal, label, contentPreview: parts.length > 0 ? parts.join(" · ") : undefined };
    }
    case "Region": {
      // label carries the real Rust Debug-formatted region_type (e.g.
      // "Background", "Foreground") — pipeline's own formatting, not ours.
      const label = raw.label || fallbackLabel(raw);
      if (raw.content) parts.push(raw.content);
      const conf = confidencePct(raw);
      if (conf) parts.push(`conf ${conf}`);
      return { isReal, label, contentPreview: parts.length > 0 ? parts.join(" · ") : undefined };
    }
    case "Text": {
      if (raw.content) parts.push(raw.content.length > 80 ? `${raw.content.slice(0, 77)}...` : raw.content);
      const lang = propStr(raw, "language");
      if (lang) parts.push(lang);
      return { isReal, label: "Text", contentPreview: parts.length > 0 ? parts.join(" · ") : undefined };
    }
    case "Face": {
      const conf = confidencePct(raw);
      if (conf) parts.push(`conf ${conf}`);
      const attrEntries = Object.entries(raw.properties ?? {});
      if (attrEntries.length > 0) {
        const kv = attrEntries.map(([k, v]) => `${k}: ${JSON.stringify(v)}`).join(", ");
        parts.push(kv.length > 100 ? `${kv.slice(0, 97)}...` : kv);
      }
      return { isReal, label: "Face", contentPreview: parts.length > 0 ? parts.join(" · ") : undefined };
    }
    case "Color": {
      const label = raw.label || fallbackLabel(raw);
      const pct = propNum(raw, "percentage");
      if (pct !== undefined) parts.push(`${(pct * 100).toFixed(1)}%`);
      const hex = propStr(raw, "hex");
      if (hex) parts.push(hex);
      return { isReal, label, contentPreview: parts.length > 0 ? parts.join(" · ") : undefined };
    }
    default: {
      // Schema-only (Composition/Quality) or unknown: no real construction
      // site to trust — surface only what's literally present, truncated.
      const entries = Object.entries(raw.properties ?? {});
      if (entries.length > 0) {
        const kv = entries.map(([k, v]) => `${k}: ${JSON.stringify(v)}`).join(", ");
        parts.push(kv.length > 120 ? `${kv.slice(0, 117)}...` : kv);
      }
      return { isReal, label: raw.label || fallbackLabel(raw), contentPreview: parts.length > 0 ? parts.join(" · ") : undefined };
    }
  }
}

// ── visual ─────────────────────────────────────────────────────────────────

const BASE = MODALITY_COLOR.image;

function visual(node: GraphViewNode): NodeVisual {
  if (!node.isReal || !isRealNodeType(node.nodeType)) {
    return {
      shape: "circle",
      radius: 6,
      fill: BASE,
      opacity: 0.4,
      stroke: BASE,
      strokeWidth: 1.5,
      strokeDasharray: "3 2",
    };
  }
  switch (node.nodeType) {
    case "Image":
      return { shape: "square", radius: 10, fill: BASE, glyph: "I" };
    case "Region":
      return { shape: "hexagon", radius: 8, fill: BASE, glyph: "R" };
    case "Text":
      return { shape: "diamond", radius: 7, fill: BASE, glyph: "T" };
    case "Face":
      return { shape: "triangle", radius: 8, fill: BASE, glyph: "F" };
    case "Color": {
      // Real touch, not decorative: fill the node with the actual detected
      // hex color when present, rather than the generic modality hue — the
      // node's own real data IS a color.
      const hex = propStr(node.raw, "hex");
      return { shape: "circle", radius: 7, fill: hex ?? BASE, stroke: "#0a0f1c", strokeWidth: 1 };
    }
    default: {
      // Object — radius scales gently with real confidence when present
      // (clamped 0..1 -> 6..9), fixed size when absent.
      const conf = typeof node.confidence === "number" ? node.confidence : undefined;
      const radius = conf === undefined ? 7 : 6 + Math.min(Math.max(conf, 0), 1) * 3;
      return { shape: "circle", radius, fill: BASE, glyph: "O" };
    }
  }
}

export const imageNodeRenderer: ModalityNodeRenderer = {
  modality: "image",
  classify,
  visual,
};
