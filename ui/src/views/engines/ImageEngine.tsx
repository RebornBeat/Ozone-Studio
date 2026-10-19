/**
 * Image Engine — structural graph view for the Image modality (pipeline 102).
 *
 * Composition over already-landed real pieces, same philosophy as
 * `CodeEngine.tsx`/`CodeCallGraph.tsx`: all data comes from
 * `../../graphViewData`'s `loadGraphData(projectId, cb, {modality:"image"})`
 * (live-updated over the real B19 WebSocket client), already converted to
 * `GraphViewNode`/`GraphViewEdge` via `../../graphRenderers/imageNodes.ts` +
 * `imageEdges.ts`'s real classifiers — this view does not re-derive
 * label/contentPreview/isReal itself, it only groups and presents what
 * `classifyNode`/`classifyEdge` already computed.
 *
 * Ground truth (confirmed against `assets/pipelines/modalities/image/main.rs`
 * via `imageNodes.ts`/`imageEdges.ts`'s own header comments, not re-verified
 * here): 6 real, construction-verified node types — Image, Object, Region,
 * Text, Face, Color (Composition/Quality are schema-only, never built).
 * Every real node except Color carries a real `bounding_box`; Object/Region/
 * Face/Color carry a real `confidence`. Real edges: `Contains` (root Image ->
 * every other node) and, pairwise between Object nodes only, `Overlaps`
 * (real bounding-box IoU) or exactly one of `LeftOf`/`RightOf`/`Above`/
 * `Below` (nearest-axis position). Every other declared edge/node type is
 * schema-only (`isReal: false`) and shown honestly, not hidden — same
 * doctrine as `CodeCallGraph.tsx`'s same-file-resolution banner.
 *
 * No "affordance" (CanSupport/CanContain) concept exists anywhere in the
 * real backend or the renderer layer — the operator's pasted spec document's
 * Section F.3 describes affordances, but nothing here constructs one, so
 * this view does not attempt to show them.
 */
import React, { useEffect, useMemo, useState } from "react";
import { loadGraphData } from "../../graphViewData";
import { GraphViewEdge, GraphViewNode, GraphViewStatus } from "../../graphViewTypes";
import { navigateTo } from "../../navigation";

const C_TEXT = "var(--color-text)";
const C_BODY = "var(--color-text-secondary)";
const C_MUTED = "var(--color-text-muted)";
const C_BORDER = "var(--color-border-faint)";
const C_PANEL = "var(--color-bg)";
const C_ERROR = "#ff8a8a";
const C_ACCENT = "#e585d8"; // MODALITY_COLOR.image (graphRenderers/defaults.ts)

const REAL_NODE_TYPES = ["Image", "Object", "Region", "Text", "Face", "Color"] as const;
const SPATIAL_EDGE_TYPES = new Set(["Overlaps", "LeftOf", "RightOf", "Above", "Below"]);

function boundingBoxLabel(node: GraphViewNode): string | null {
  const bb = node.raw.bounding_box as { x: number; y: number; width: number; height: number } | undefined;
  if (!bb) return null;
  return `${Math.round(bb.x)},${Math.round(bb.y)} ${Math.round(bb.width)}×${Math.round(bb.height)}`;
}

function confidenceLabel(node: GraphViewNode): string | null {
  return typeof node.confidence === "number" ? `${Math.round(node.confidence * 100)}%` : null;
}

function jumpToNode(node: GraphViewNode, projectId: number) {
  navigateTo({ kind: "graph-node", projectId, nodeId: node.id });
}

const NodeChip: React.FC<{ node: GraphViewNode; projectId: number; accent?: string }> = ({
  node,
  projectId,
  accent,
}) => (
  <div
    onClick={() => jumpToNode(node, projectId)}
    title="Jump to node in Graph View"
    style={{
      display: "flex",
      alignItems: "baseline",
      gap: 8,
      padding: "6px 8px",
      border: `1px solid ${C_BORDER}`,
      borderLeft: `3px solid ${accent ?? C_ACCENT}`,
      borderRadius: 6,
      cursor: "pointer",
      opacity: node.isReal ? 1 : 0.5,
    }}
  >
    <span style={{ color: C_TEXT, fontSize: 12.5, fontWeight: 600 }}>{node.label}</span>
    {node.contentPreview && (
      <span style={{ color: C_BODY, fontSize: 11.5 }}>{node.contentPreview}</span>
    )}
    {confidenceLabel(node) && <span style={{ color: C_MUTED, fontSize: 11 }}>conf {confidenceLabel(node)}</span>}
    {boundingBoxLabel(node) && (
      <span style={{ color: C_MUTED, fontSize: 11 }}>box {boundingBoxLabel(node)}</span>
    )}
    {!node.isReal && <span style={{ color: C_MUTED, fontSize: 10.5 }}>(schema-only — not constructed)</span>}
  </div>
);

/** Real spatial relationships (Overlaps/LeftOf/RightOf/Above/Below) touching
 * a given Object node, in either direction — these are the only real
 * pairwise edges this pipeline ever constructs (Contains is root->child
 * only, shown separately as grouping, not as a relationship list). */
function spatialRelationsFor(nodeId: string, edges: GraphViewEdge[], byId: Map<string, GraphViewNode>): string[] {
  const out: string[] = [];
  for (const e of edges) {
    if (!e.isReal || !SPATIAL_EDGE_TYPES.has(e.edgeType)) continue;
    if (e.from === nodeId) {
      const other = byId.get(e.to);
      if (other) out.push(`${e.edgeType} ${other.label}`);
    } else if (e.to === nodeId) {
      const other = byId.get(e.from);
      if (other) out.push(`${other.label} ${e.edgeType} (this)`);
    }
  }
  return out;
}

export const ImageEngine: React.FC<{ projectId: number | null }> = ({ projectId }) => {
  const [status, setStatus] = useState<GraphViewStatus>({ kind: "loading" });

  useEffect(() => {
    if (projectId === null) return;
    setStatus({ kind: "loading" });
    const dispose = loadGraphData(projectId, setStatus, { modality: "image" });
    return dispose;
  }, [projectId]);

  const grouped = useMemo(() => {
    if (status.kind !== "ready") return null;
    const byId = new Map(status.data.nodes.map((n) => [n.id, n]));
    const groups: Record<string, GraphViewNode[]> = {
      Image: [],
      Object: [],
      Region: [],
      Text: [],
      Face: [],
      Color: [],
      Other: [],
    };
    for (const n of status.data.nodes) {
      const key = (REAL_NODE_TYPES as readonly string[]).includes(n.nodeType) ? n.nodeType : "Other";
      groups[key].push(n);
    }
    return { byId, groups, edges: status.data.edges };
  }, [status]);

  if (projectId === null) {
    return <div style={{ color: C_MUTED, fontSize: 12.5 }}>Select a project to see its image graph.</div>;
  }

  return (
    <div>
      <div style={{ fontSize: 12.5, fontWeight: 700, color: C_TEXT, marginBottom: 8 }}>Image Engine</div>

      {status.kind === "loading" && <div style={{ color: C_MUTED, fontSize: 12.5 }}>Loading image graph…</div>}
      {status.kind === "error" && (
        <div style={{ color: C_ERROR, fontSize: 12.5 }}>Error loading image graph: {status.message}</div>
      )}
      {status.kind === "empty" && (
        <div style={{ color: C_MUTED, fontSize: 12.5 }}>
          No image-modality graph exists for this project yet — link an image file to see its structure here.
        </div>
      )}

      {status.kind === "ready" && grouped && (
        <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
          {grouped.groups.Image.map((img) => (
            <div
              key={img.id}
              style={{ border: `1px solid ${C_BORDER}`, borderRadius: 8, padding: "8px 10px", background: C_PANEL }}
            >
              <div style={{ color: C_ACCENT, fontWeight: 700, fontSize: 12.5 }}>▧ {img.label}</div>
              {img.contentPreview && (
                <div style={{ color: C_BODY, fontSize: 11.5, marginTop: 2 }}>{img.contentPreview}</div>
              )}
            </div>
          ))}

          {([
            ["Object", "Objects"],
            ["Region", "Regions"],
            ["Face", "Faces"],
            ["Text", "Text (OCR)"],
            ["Color", "Dominant colors"],
          ] as const).map(([key, title]) => {
            const nodes = grouped.groups[key];
            if (nodes.length === 0) return null;
            return (
              <div key={key}>
                <div style={{ color: C_MUTED, fontSize: 11, fontWeight: 700, marginBottom: 6, textTransform: "uppercase", letterSpacing: 0.4 }}>
                  {title} ({nodes.length})
                </div>
                <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                  {nodes.map((n) => (
                    <div key={n.id}>
                      <NodeChip
                        node={n}
                        projectId={projectId}
                        accent={key === "Color" ? ((n.raw.properties?.hex as string) || C_ACCENT) : undefined}
                      />
                      {key === "Object" &&
                        (() => {
                          const rels = spatialRelationsFor(n.id, grouped.edges, grouped.byId);
                          return rels.length > 0 ? (
                            <div style={{ marginLeft: 16, marginTop: 2, color: C_MUTED, fontSize: 11 }}>
                              {rels.join(" · ")}
                            </div>
                          ) : null;
                        })()}
                    </div>
                  ))}
                </div>
              </div>
            );
          })}

          {grouped.groups.Other.length > 0 && (
            <div>
              <div style={{ color: C_MUTED, fontSize: 11, fontWeight: 700, marginBottom: 6, textTransform: "uppercase", letterSpacing: 0.4 }}>
                Unclassified / schema-only ({grouped.groups.Other.length})
              </div>
              <div style={{ color: C_MUTED, fontSize: 11, lineHeight: 1.6 }}>
                Declared node types with no real construction site in the current pipeline (e.g. Composition,
                Quality) — shown dimmed, never hidden.
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 4, marginTop: 6 }}>
                {grouped.groups.Other.map((n) => (
                  <NodeChip key={n.id} node={n} projectId={projectId} />
                ))}
              </div>
            </div>
          )}

          {grouped.groups.Image.length === 0 &&
            grouped.groups.Object.length === 0 &&
            grouped.groups.Region.length === 0 &&
            grouped.groups.Face.length === 0 &&
            grouped.groups.Text.length === 0 &&
            grouped.groups.Color.length === 0 &&
            grouped.groups.Other.length === 0 && (
              <div style={{ color: C_MUTED, fontSize: 12.5 }}>Image graph has no nodes.</div>
            )}
        </div>
      )}
    </div>
  );
};

export default ImageEngine;
