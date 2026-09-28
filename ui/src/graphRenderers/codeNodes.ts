/**
 * C2 — Code modality node renderer.
 *
 * Ground truth (assets/pipelines/modalities/code/main.rs, `build_graph_nodes_edges`):
 * `CodeNodeType` declares 11 variants but only 4 are ever CONSTRUCTED —
 * File (2622), Function (2639), Class (2674), Import (2765). Module, Method,
 * Variable, Type, Export, Parameter, Block are schema-only (matches
 * docs/GRAPH_RELATIONSHIP_REGISTRY.md §1). Any node whose type is not one of
 * the 4 is classified `isReal: false` and drawn dashed/dimmed — never hidden,
 * never styled as real.
 *
 * Real node fields: `name` (File: file path; Function/Class: identifier;
 * Import: module path), optional `position {start_line,end_line}`, and
 * `properties` — File{language,line_count}, Function{is_async,is_public,
 * complexity}, Class{is_public,extends?}, Import{is_external,items}. Code
 * nodes carry NO source text, so `contentPreview` is a compact summary built
 * only from those real fields (absent fields are omitted, not defaulted).
 */
import type { GraphViewNode, RawGraphNode } from "../graphViewTypes";
import type { ModalityNodeRenderer, NodeClassification, NodeVisual } from "./types";
import { MODALITY_COLOR } from "./defaults";

const REAL_NODE_TYPES: readonly string[] = ["File", "Function", "Class", "Import"];

function isRealNodeType(nodeType: string): boolean {
  return REAL_NODE_TYPES.includes(nodeType);
}

// ── Typed readers over the loosely-typed raw payload ──────────────────────

function prop(raw: RawGraphNode, key: string): unknown {
  const props = raw.properties;
  return props && typeof props === "object" ? props[key] : undefined;
}
function propBool(raw: RawGraphNode, key: string): boolean | undefined {
  const v = prop(raw, key);
  return typeof v === "boolean" ? v : undefined;
}
function propNum(raw: RawGraphNode, key: string): number | undefined {
  const v = prop(raw, key);
  return typeof v === "number" && Number.isFinite(v) ? v : undefined;
}
function propStr(raw: RawGraphNode, key: string): string | undefined {
  const v = prop(raw, key);
  return typeof v === "string" && v.length > 0 ? v : undefined;
}
function propStrList(raw: RawGraphNode, key: string): string[] {
  const v = prop(raw, key);
  return Array.isArray(v) ? v.filter((x): x is string => typeof x === "string") : [];
}

function lineRange(raw: RawGraphNode): string | undefined {
  const p = raw.position;
  if (!p || typeof p !== "object") return undefined;
  const { start_line: s, end_line: e } = p as { start_line?: unknown; end_line?: unknown };
  if (typeof s !== "number") return undefined;
  return typeof e === "number" && e !== s ? `L${s}-${e}` : `L${s}`;
}

function basename(path: string): string {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts.length > 0 ? parts[parts.length - 1] : path;
}

function fallbackLabel(raw: RawGraphNode): string {
  return `${raw.node_type} #${raw.node_id}`;
}

// ── classify ───────────────────────────────────────────────────────────────

function classify(raw: RawGraphNode): NodeClassification {
  const name = typeof raw.name === "string" ? raw.name : "";
  const isReal = isRealNodeType(raw.node_type);
  const parts: string[] = [];

  switch (raw.node_type) {
    case "File": {
      if (name) parts.push(name);
      const lang = propStr(raw, "language");
      if (lang) parts.push(lang);
      const lines = propNum(raw, "line_count");
      if (lines !== undefined) parts.push(`${lines} lines`);
      return {
        isReal,
        label: name ? basename(name) : fallbackLabel(raw),
        contentPreview: parts.length > 0 ? parts.join(" · ") : undefined,
      };
    }
    case "Function": {
      if (propBool(raw, "is_async")) parts.push("async");
      const pub = propBool(raw, "is_public");
      if (pub !== undefined) parts.push(pub ? "public" : "private");
      const cx = propNum(raw, "complexity");
      if (cx !== undefined) parts.push(`complexity ${cx}`);
      const range = lineRange(raw);
      if (range) parts.push(range);
      return {
        isReal,
        label: name || fallbackLabel(raw),
        contentPreview: parts.length > 0 ? parts.join(" · ") : undefined,
      };
    }
    case "Class": {
      const pub = propBool(raw, "is_public");
      if (pub !== undefined) parts.push(pub ? "public" : "private");
      const ext = propStr(raw, "extends");
      if (ext) parts.push(`extends ${ext}`);
      const range = lineRange(raw);
      if (range) parts.push(range);
      return {
        isReal,
        label: name || fallbackLabel(raw),
        contentPreview: parts.length > 0 ? parts.join(" · ") : undefined,
      };
    }
    case "Import": {
      const external = propBool(raw, "is_external");
      if (external !== undefined) parts.push(external ? "external" : "internal");
      const items = propStrList(raw, "items");
      if (items.length > 0) {
        const shown = items.slice(0, 6).join(", ");
        parts.push(items.length > 6 ? `{${shown}, +${items.length - 6} more}` : `{${shown}}`);
      }
      const range = lineRange(raw);
      if (range) parts.push(range);
      return {
        isReal,
        label: name || fallbackLabel(raw),
        contentPreview: parts.length > 0 ? parts.join(" · ") : undefined,
      };
    }
    default: {
      // Schema-only / unknown type: no real construction site exists, so no
      // per-type field knowledge to trust. Surface only what is literally
      // present (position + raw property keys/values), truncated.
      const range = lineRange(raw);
      if (range) parts.push(range);
      const entries = Object.entries(raw.properties ?? {});
      if (entries.length > 0) {
        const kv = entries.map(([k, v]) => `${k}: ${JSON.stringify(v)}`).join(", ");
        parts.push(kv.length > 120 ? `${kv.slice(0, 117)}...` : kv);
      }
      return {
        isReal,
        label: name || fallbackLabel(raw),
        contentPreview: parts.length > 0 ? parts.join(" · ") : undefined,
      };
    }
  }
}

// ── visual ─────────────────────────────────────────────────────────────────

const BASE = MODALITY_COLOR.code;

function visual(node: GraphViewNode): NodeVisual {
  if (!node.isReal || !isRealNodeType(node.nodeType)) {
    // Honest "schema-only / unknown type" state: same modality hue, but
    // dashed outline + heavy dimming so it can't be mistaken for a real type.
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
    case "File":
      return { shape: "square", radius: 9, fill: BASE, glyph: "F" };
    case "Class":
      return { shape: "hexagon", radius: 9, fill: BASE, glyph: "C" };
    case "Import":
      return { shape: "diamond", radius: 7, fill: BASE, glyph: "I" };
    default: {
      // Function — radius scales gently with the real `complexity` property
      // (clamped 1..10 → 6.3..9) when present; fixed size when absent.
      const cx = propNum(node.raw, "complexity");
      const radius = cx === undefined ? 7 : 6 + Math.min(Math.max(cx, 1), 10) * 0.3;
      return { shape: "circle", radius, fill: BASE, glyph: "ƒ" };
    }
  }
}

export const codeNodeRenderer: ModalityNodeRenderer = {
  modality: "code",
  classify,
  visual,
};
