/**
 * GraphKit — GraphCanvas: THE one canvas component (guide §4.2). Layout is a
 * prop, not N canvases. Hand-rolled SVG, no physics library — deterministic
 * layouts that are honest about what they are: `grid` groups by modality,
 * `radial` rings one modality per ring, `tree` layers by structural depth
 * from root nodes (edges typed in a lens's `structuralEdges`). Node visuals
 * come from the existing classifier palette (MODALITY_COLOR); edges carry
 * the edgeClass honesty (real solid / derived dashed / inferred dotted).
 *
 * Deliberately NOT a force simulation: a physics lib is a leaf renderer that
 * can sit behind this contract later; the substrate contracts (nodes, edges,
 * selection, navigation) are the stable part.
 */
import React, { useMemo } from "react";
import { MODALITY_COLOR } from "../graphRenderers/defaults";
import type { GraphViewEdge, GraphViewNode, Modality } from "../graphViewTypes";

export type CanvasLayout = "grid" | "radial" | "tree";

export interface GraphCanvasProps {
  nodes: GraphViewNode[];
  edges: GraphViewEdge[];
  layout?: CanvasLayout;
  width?: number;
  height?: number;
  selectedId?: string | null;
  onSelect?: (id: string) => void;
  /** Structural edge types used by the tree layout to compute depth
   * (defaults to a broad structural set; other layouts ignore this). */
  structuralEdges?: string[];
}

const W = 760;
const H = 420;
const NODE_R = 9;

function edgeDash(cls?: string): string | undefined {
  if (cls === "inferred") return "1 3";
  if (cls === "derived" || cls === "cross-modal") return "4 3";
  return undefined; // real = solid
}

export const GraphCanvas: React.FC<GraphCanvasProps> = ({
  nodes,
  edges,
  layout = "grid",
  width = W,
  height = H,
  selectedId,
  onSelect,
  structuralEdges = ["Contains", "ParentOf", "Calls", "InCollection", "Follows"],
}) => {
  const positions = useMemo(() => {
    const pos = new Map<string, { x: number; y: number }>();
    if (nodes.length === 0) return pos;

    if (layout === "grid") {
      // Group by modality, rows by group — modality legible at a glance.
      const byModality = new Map<string, GraphViewNode[]>();
      for (const n of nodes) {
        const key = n.modality ?? "container";
        (byModality.get(key) ?? byModality.set(key, []).get(key)!).push(n);
      }
      const groups = Array.from(byModality.entries());
      const rows = groups.length;
      groups.forEach(([m, list], row) => {
        list.forEach((n, i) => {
          const cols = Math.max(list.length, 1);
          pos.set(n.id, {
            x: 40 + ((i + 0.5) / cols) * (width - 80),
            y: 30 + ((row + 0.5) / rows) * (height - 60),
          });
          void m;
        });
      });
      return pos;
    }

    if (layout === "radial") {
      // One ring per modality, centered — the modality wheel as a layout.
      const byModality = new Map<string, GraphViewNode[]>();
      for (const n of nodes) {
        const key = n.modality ?? "container";
        (byModality.get(key) ?? byModality.set(key, []).get(key)!).push(n);
      }
      const groups = Array.from(byModality.entries());
      const cx = width / 2;
      const cy = height / 2;
      groups.forEach(([m, list], ring) => {
        const r = 28 + ring * Math.min(52, (Math.min(width, height) / 2 - 40) / Math.max(groups.length - 1, 1));
        list.forEach((n, i) => {
          const a = (i / Math.max(list.length, 1)) * Math.PI * 2 + ring * 0.5;
          pos.set(n.id, { x: cx + Math.cos(a) * r, y: cy + Math.sin(a) * r });
        });
        void m;
      });
      return pos;
    }

    // tree: BFS layers from roots (nodes with no incoming structural edge),
    // children laid out under parents. Cycle-guarded like every tree build.
    const structural = new Set(structuralEdges);
    const childrenOf = new Map<string, string[]>();
    const hasParent = new Set<string>();
    for (const e of edges) {
      if (!structural.has(e.edgeType)) continue;
      (childrenOf.get(e.from) ?? childrenOf.set(e.from, []).get(e.from)!).push(e.to);
      hasParent.add(e.to);
    }
    const byId = new Map(nodes.map((n) => [n.id, n]));
    const roots = nodes.filter((n) => !hasParent.has(n.id)).map((n) => n.id);
    const depthOf = new Map<string, number>();
    const queue: string[] = [];
    for (const r of roots.length > 0 ? roots : nodes.slice(0, 1).map((n) => n.id)) {
      if (!depthOf.has(r)) {
        depthOf.set(r, 0);
        queue.push(r);
      }
    }
    // Reachable-by-structure nodes get depths; unreachable append at the end.
    for (let qi = 0; qi < queue.length; qi++) {
      const id = queue[qi];
      const d = depthOf.get(id) ?? 0;
      for (const c of childrenOf.get(id) ?? []) {
        if (!depthOf.has(c) && byId.has(c)) {
          depthOf.set(c, d + 1);
          queue.push(c);
        }
      }
    }
    for (const n of nodes) if (!depthOf.has(n.id)) depthOf.set(n.id, 999);
    const byDepth = new Map<number, string[]>();
    for (const n of nodes) {
      const d = depthOf.get(n.id) ?? 999;
      (byDepth.get(d) ?? byDepth.set(d, []).get(d)!).push(n.id);
    }
    const layers = Array.from(byDepth.keys()).sort((a, b) => a - b);
    layers.forEach((d, li) => {
      const list = byDepth.get(d)!;
      list.forEach((id, i) => {
        pos.set(id, {
          x: 40 + ((i + 0.5) / Math.max(list.length, 1)) * (width - 80),
          y: 26 + ((li + 0.5) / layers.length) * (height - 52),
        });
      });
    });
    return pos;
  }, [nodes, edges, layout, width, height, structuralEdges]);

  const glyph = (n: GraphViewNode): string =>
    (n.modality ?? n.nodeType ?? "?").toString().charAt(0).toUpperCase();

  return (
    <svg
      width="100%"
      viewBox={`0 0 ${width} ${height}`}
      style={{ background: "var(--color-bg)", border: "1px solid var(--color-border-faint)", borderRadius: 8 }}
      role="img"
      aria-label={`graph canvas: ${nodes.length} nodes, ${edges.length} edges, ${layout} layout`}
    >
      {edges.map((e) => {
        const a = positions.get(e.from);
        const b = positions.get(e.to);
        if (!a || !b) return null;
        return (
          <line
            key={e.id}
            x1={a.x}
            y1={a.y}
            x2={b.x}
            y2={b.y}
            stroke={e.edgeClass === "cross-modal" ? "#e879f9" : "var(--color-border)"}
            strokeWidth={e.edgeClass === "cross-modal" ? 2 : 1}
            strokeDasharray={edgeDash(e.edgeClass)}
            opacity={0.7}
          />
        );
      })}
      {nodes.map((n) => {
        const p = positions.get(n.id);
        if (!p) return null;
        const color =
          n.modality && n.modality in MODALITY_COLOR
            ? MODALITY_COLOR[n.modality as Modality]
            : "var(--color-text-muted)";
        const selected = n.id === selectedId;
        return (
          <g
            key={n.id}
            onClick={() => onSelect?.(n.id)}
            style={{ cursor: onSelect ? "pointer" : "default" }}
          >
            {selected && <circle cx={p.x} cy={p.y} r={NODE_R + 4} fill="none" stroke="#4fc3f7" strokeWidth={2} />}
            <circle cx={p.x} cy={p.y} r={NODE_R} fill={color} fillOpacity={n.isReal === false ? 0.45 : 0.9} />
            <text
              x={p.x}
              y={p.y + 3.5}
              textAnchor="middle"
              fontSize={9}
              fontWeight={700}
              fill="var(--color-bg)"
              pointerEvents="none"
            >
              {glyph(n)}
            </text>
            <title>{n.label}</title>
          </g>
        );
      })}
      {nodes.length === 0 && (
        <text x={width / 2} y={height / 2} textAnchor="middle" fontSize={12} fill="var(--color-text-muted)">
          no nodes in this slice
        </text>
      )}
    </svg>
  );
};

export default GraphCanvas;
