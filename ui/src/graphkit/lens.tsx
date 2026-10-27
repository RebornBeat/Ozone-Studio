/**
 * GraphKit — engine lenses. An engine becomes a DECLARATION over shared
 * chrome, not a private fetch (guide §4.5). The generic lens covers every
 * modality that has no dedicated view yet — the 27-modality fleet is
 * reachable day one; specialized lenses port incrementally from the 8
 * existing engine components (their tree-builders become layout fns).
 *
 * This module is the ONE registry: Panels and the workspace both render
 * from it (the two-books defect stays dead — the engine components remain
 * the specialized detail surfaces and are mounted BY a lens's `render`).
 */
import React from "react";
import type { Modality } from "../graphViewTypes";
import { MODALITY_COLOR, modalityFromObjectStorePath } from "../graphRenderers/defaults";
import { GraphCanvas } from "./GraphCanvas";

export type LensLayout = "tree" | "force" | "radial" | "table";

export interface LensDefinition {
  id: LensIdString;
  label: string;
  modality: Modality | "all";
  layout: LensLayout;
  /** Structural edge types this lens groups/trees by (e.g. 3D's
   * Contains/ParentOf/InCollection). Undefined = the lens is tabular. */
  structuralEdges?: string[];
  /** Render the lens body for a ready substrate. Receives the modality-
   * filtered slice, so a lens never fetches. */
  render: React.FC<LensBodyProps>;
}

export interface LensBodyProps {
  projectId: number;
  nodes: import("../graphViewTypes").GraphViewNode[];
  edges: import("../graphViewTypes").GraphViewEdge[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}

export type LensIdString = string;

export function defineLens(d: LensDefinition): LensDefinition {
  return d;
}

/** The generic lens: an honest typed table + the shared inspector pattern —
 * every modality without a dedicated view is covered by this. It renders
 * ONLY real fields present on nodes (no invented columns). */
export function genericLensFor(modality: Modality): LensDefinition {
  return defineLens({
    id: `lens-${modality.toLowerCase()}`,
    label: modality,
    modality,
    layout: "table",
    render: ({ nodes, edges, selectedId, onSelect }) => {
      const cols = new Set<string>();
      for (const n of nodes.slice(0, 50)) {
        for (const k of Object.keys((n.raw ?? {}) as Record<string, unknown>)) {
          if (k !== "node_id" && k !== "node_type") cols.add(k);
        }
      }
      const outDegree = new Map<string, number>();
      for (const e of edges) outDegree.set(e.from, (outDegree.get(e.from) ?? 0) + 1);
      return (
        <div style={{ display: "flex", flexDirection: "column", gap: 10, minWidth: 0 }}>
          {/* GraphCanvas (wave 6): every generic lens gets the real canvas —
              the modality's nodes/edges rendered, selectable, never a
              table-only second-class view. */}
          <GraphCanvas
            nodes={nodes}
            edges={edges}
            layout="grid"
            selectedId={selectedId}
            onSelect={onSelect}
            height={260}
          />
          <div style={{ display: "flex", gap: 12, minWidth: 0 }}>
          <div style={{ flex: "1 1 60%", minWidth: 0 }}>
            <div
              style={{
                color: "var(--color-text-muted)",
                fontSize: 10.5,
                textTransform: "uppercase",
                letterSpacing: 0.4,
                marginBottom: 4,
              }}
            >
              {modality} nodes ({nodes.length}) · edges ({edges.length})
            </div>
            {nodes.length === 0 ? (
              <div style={{ color: "var(--color-text-muted)", fontSize: 11.5 }}>
                No {modality} nodes in this project's graph yet.
              </div>
            ) : (
              nodes.map((n) => {
                const raw = (n.raw ?? {}) as Record<string, unknown>;
                return (
                  <div
                    key={n.id}
                    onClick={() => onSelect(n.id)}
                    style={{
                      display: "flex",
                      gap: 8,
                      padding: "2px 6px",
                      borderRadius: 4,
                      cursor: "pointer",
                      background: n.id === selectedId ? "rgba(79,195,247,0.12)" : "transparent",
                      fontSize: 11.5,
                      color: "var(--color-text-secondary)",
                      alignItems: "baseline",
                      minWidth: 0,
                    }}
                  >
                    <span style={{ color: n.id === selectedId ? MODALITY_COLOR[modality] : undefined }}>
                      {String(raw.node_type ?? n.nodeType)}
                    </span>
                    <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                      {n.label}
                    </span>
                    {(outDegree.get(n.id) ?? 0) > 0 && (
                      <span style={{ color: "var(--color-text-muted)", fontSize: 10 }}>
                        {outDegree.get(n.id)} edge{outDegree.get(n.id) === 1 ? "" : "s"}
                      </span>
                    )}
                  </div>
                );
              })
            )}
          </div>
          <div style={{ flex: "1 1 40%", minWidth: 0 }}>
            {nodes
              .filter((n) => n.id === selectedId)
              .map((n) => {
                const raw = (n.raw ?? {}) as Record<string, unknown>;
                return (
                  <div
                    key={n.id}
                    style={{
                      border: "1px solid var(--color-border-faint)",
                      borderRadius: 8,
                      padding: "8px 10px",
                    }}
                  >
                    <div style={{ color: MODALITY_COLOR[modality], fontWeight: 700, fontSize: 12.5, marginBottom: 4 }}>
                      {n.label}
                    </div>
                    {Object.entries(raw)
                      .filter(([k, v]) => k !== "node_id" && v !== null && v !== "" && typeof v !== "object")
                      .map(([k, v]) => (
                        <div key={k} style={{ display: "flex", gap: 8, fontSize: 11.5, marginTop: 3 }}>
                          <span style={{ color: "var(--color-text-muted)", minWidth: 90 }}>{k}</span>
                          <span style={{ color: "var(--color-text-secondary)" }}>{String(v)}</span>
                        </div>
                      ))}
                  </div>
                );
              })}
            {selectedId && !nodes.some((n) => n.id === selectedId) && (
              <div style={{ color: "var(--color-text-muted)", fontSize: 12, padding: "8px 2px" }}>
                Select a node to inspect it.
              </div>
            )}
          </div>
          </div>
        </div>
      );
    },
  });
}

/** The modalities with DEDICATED engine views today (their components are
 * the lens detail surfaces; adoption into this registry is the ports step).
 * Everything else in the palette gets `genericLensFor` — nothing invisible. */
export const DEDICATED_ENGINE_MODALITIES: Modality[] = [
  "code",
  "math",
  "text",
  "image",
  "3D",
  "audio",
  "video",
  "geospatial",
  "CAD",
];

export function lensExistsFor(modality: Modality): boolean {
  return DEDICATED_ENGINE_MODALITIES.includes(modality);
}

export { modalityFromObjectStorePath };
