/**
 * D1 — Fabric View: cross-modality spatial map of a project's persisted graphs.
 *
 * One cluster per modality (code / math / text), sized by the REAL node count of
 * that modality's ModalityGraph containers, coloured with MODALITY_COLOR, and
 * linked ONLY by real cross-modal signals. Nothing here is fabricated.
 *
 * Real sources (all confirmed against the live host + source):
 *  - B1: GetContainer{project} -> global_state.child_ids -> per-child GetContainer, keep
 *    container_type == "ModalityGraph" -> GetContainerContent (B0) -> Content.json
 *    {graph_id, nodes, edges, ...}. Modality comes from storage.object_store_path
 *    ("graphs/{code|math|text}_*.json"): local_state.metadata.modality is "Structured" for math
 *    (the shared Modality enum has no Math variant), so it is not a reliable discriminant.
 *  - B16: container-level `SimilarTo` relations live in local_state.context.relationships of each
 *    ModalityGraph container. A link is drawn between two clusters ONLY when such a relation joins
 *    graph containers of DIFFERENT modalities in this project. Same-modality SimilarTo and relations
 *    to containers outside this project are counted and listed, never drawn as cluster links.
 *  - In-graph edges whose renderer classifies them "cross-modal" (classifyEdge) are counted per
 *    cluster. Their target modality is not recorded on the edge, so they never become a link.
 *    (At the time of writing text's `is_cross_modal` is hardcoded false, so this is expected to be 0.)
 *
 * The zsei envelope: the Electron bridge and the direct-HTTP fallback both return the full
 * `{success, result, error}` body, so `unwrap` accepts either that or an already-unwrapped result.
 *
 * Live updates: same invalidate-and-refetch model as graphViewData.ts (frames are bare
 * change-notifications), via the shared graph-event client.
 */
import React, { useEffect, useMemo, useState } from "react";
import { zseiQuery } from "../../ozoneClient";
import { getGraphEventClient, GraphEventFrame } from "../../graphEventClient";
import { classifyEdge } from "../../graphRenderers";
import { MODALITY_COLOR } from "../../graphRenderers/defaults";
import { navigateTo } from "../../navigation";
import type { ContainerRelation, Modality, RawGraphEdge, RawGraphNode } from "../../graphViewTypes";

export type FabricViewProps = { projectId: number | null };

const C_TEXT = "var(--color-text)";
const C_BODY = "var(--color-text-secondary)";
const C_MUTED = "var(--color-text-muted)";
const C_BORDER = "var(--color-border-faint)";
const C_PANEL = "var(--color-bg)";
const C_WARN = "#e8c14f";

const MODALITIES: Modality[] = ["code", "math", "text"];

// ── Data ─────────────────────────────────────────────────────────────────

interface GraphSummary {
  containerId: number;
  modality: Modality;
  /** False when the container exists but its content file did not resolve to JSON. */
  readable: boolean;
  nodeCount: number;
  edgeCount: number;
  nodeTypes: Record<string, number>;
  edgeTypes: Record<string, number>;
  /** In-graph edges the modality renderer classifies as cross-modal. */
  crossModalEdges: number;
  relations: ContainerRelation[];
}

interface SimilarLink {
  fromContainer: number;
  toContainer: number;
  fromModality: Modality;
  /** null when the target is not one of this project's graph containers. */
  toModality: Modality | null;
  confidence?: number;
  discoveredVia?: string;
}

interface FabricData {
  graphs: GraphSummary[];
  similar: SimilarLink[];
}

type Status =
  | { kind: "idle" }
  | { kind: "loading" }
  | { kind: "error"; message: string }
  | { kind: "empty" }
  | { kind: "ready"; data: FabricData };

function unwrap(r: any): any {
  if (r && typeof r === "object" && "success" in r && "result" in r) {
    if (r.success === false) throw new Error(typeof r.error === "string" ? r.error : "zsei query failed");
    return r.result;
  }
  return r;
}

async function query(q: Record<string, unknown>): Promise<any> {
  return unwrap(await zseiQuery<any>(q));
}

async function getContainer(id: number): Promise<any | null> {
  const r = await query({ GetContainer: { container_id: id } });
  return r && typeof r === "object" && "Container" in r ? r.Container : null;
}

function modalityFromPath(path: string | null | undefined): Modality | null {
  if (!path) return null;
  if (path.startsWith("graphs/code_")) return "code";
  if (path.startsWith("graphs/math_")) return "math";
  if (path.startsWith("graphs/text_")) return "text";
  return null;
}

function tally(values: string[]): Record<string, number> {
  const out: Record<string, number> = {};
  for (const v of values) out[v] = (out[v] ?? 0) + 1;
  return out;
}

async function loadFabricData(projectId: number): Promise<FabricData | null> {
  const project = await getContainer(projectId);
  if (!project) throw new Error(`Project container ${projectId} not found`);

  const childIds: number[] = project.global_state?.child_ids ?? [];
  const children = await Promise.all(childIds.map((id) => getContainer(id)));

  const graphContainers = children
    .map((c, i) => ({ c, id: childIds[i] }))
    .filter((x) => x.c && x.c.local_state?.metadata?.container_type === "ModalityGraph")
    .map((x) => ({ ...x, modality: modalityFromPath(x.c.local_state?.storage?.object_store_path) }))
    .filter((x): x is typeof x & { modality: Modality } => x.modality !== null);

  if (graphContainers.length === 0) return null;

  const graphs: GraphSummary[] = [];
  for (const g of graphContainers) {
    const content = (await query({ GetContainerContent: { container_id: g.id } }))?.Content;
    const json = content?.json as { nodes?: RawGraphNode[]; edges?: RawGraphEdge[] } | null | undefined;
    const nodes = json?.nodes ?? [];
    const edges = json?.edges ?? [];
    graphs.push({
      containerId: g.id,
      modality: g.modality,
      readable: !!json,
      nodeCount: nodes.length,
      edgeCount: edges.length,
      nodeTypes: tally(nodes.map((n) => n.node_type)),
      edgeTypes: tally(edges.map((e) => e.edge_type)),
      crossModalEdges: edges.filter((e) => classifyEdge(g.modality, e).edgeClass === "cross-modal").length,
      relations: (g.c.local_state?.context?.relationships ?? []) as ContainerRelation[],
    });
  }

  const modalityByContainer = new Map(graphs.map((g) => [g.containerId, g.modality] as const));
  const similar: SimilarLink[] = [];
  for (const g of graphs) {
    for (const r of g.relations) {
      if (r.relation_type !== "SimilarTo") continue;
      similar.push({
        fromContainer: g.containerId,
        toContainer: r.target_id,
        fromModality: g.modality,
        toModality: modalityByContainer.get(r.target_id) ?? null,
        confidence: r.confidence,
        discoveredVia: r.discovered_via,
      });
    }
  }
  return { graphs, similar };
}

function useFabricData(projectId: number | null): Status {
  const [status, setStatus] = useState<Status>({ kind: "idle" });

  useEffect(() => {
    if (projectId === null) {
      setStatus({ kind: "idle" });
      return;
    }
    let cancelled = false;
    const fetchOnce = async () => {
      setStatus((s) => (s.kind === "ready" ? s : { kind: "loading" }));
      try {
        const data = await loadFabricData(projectId);
        if (!cancelled) setStatus(data ? { kind: "ready", data } : { kind: "empty" });
      } catch (e) {
        if (!cancelled) setStatus({ kind: "error", message: e instanceof Error ? e.message : String(e) });
      }
    };
    fetchOnce();

    const client = getGraphEventClient();
    client.connect();
    const unsubscribe = client.onEvent(
      (frame: GraphEventFrame) => {
        if (frame.parent_id === projectId || frame.container_id === projectId) fetchOnce();
      },
      [`proj:${projectId}`, "scope:global"],
    );
    return () => {
      cancelled = true;
      unsubscribe();
    };
  }, [projectId]);

  return status;
}

// ── Layout (display only — a deterministic placement, not backend geometry) ──

const W = 640;
const H = 380;
const CENTERS: Record<Modality, { x: number; y: number }> = {
  code: { x: 130, y: 110 },
  math: { x: 510, y: 110 },
  text: { x: 320, y: 290 },
  image: { x: 320, y: 60 },
};
const MIN_R = 20;
const MAX_R = 74;

function clusterRadius(nodes: number, maxNodes: number): number {
  if (nodes <= 0 || maxNodes <= 0) return MIN_R;
  return MIN_R + (MAX_R - MIN_R) * Math.sqrt(nodes / maxNodes);
}

const sortedEntries = (rec: Record<string, number>): [string, number][] =>
  Object.entries(rec).sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));

// ── UI ────────────────────────────────────────────────────────────────────

const Note: React.FC<{ children: React.ReactNode; warn?: boolean }> = ({ children, warn }) => (
  <div
    style={{
      fontSize: 12,
      color: warn ? C_WARN : C_MUTED,
      border: `1px solid ${warn ? C_WARN : C_BORDER}`,
      borderRadius: 8,
      padding: "8px 10px",
      lineHeight: 1.5,
    }}
  >
    {children}
  </div>
);

const Breakdown: React.FC<{ title: string; rec: Record<string, number> }> = ({ title, rec }) => {
  const rows = sortedEntries(rec);
  return (
    <div style={{ marginBottom: 10 }}>
      <div style={{ fontSize: 11, fontWeight: 700, letterSpacing: 0.4, textTransform: "uppercase", color: C_MUTED, marginBottom: 3 }}>
        {title}
      </div>
      {rows.length === 0 ? (
        <div style={{ fontSize: 12, color: C_MUTED, fontStyle: "italic" }}>none</div>
      ) : (
        rows.map(([k, v]) => (
          <div key={k} style={{ display: "flex", justifyContent: "space-between", fontSize: 12, color: C_BODY, gap: 12 }}>
            <span style={{ overflowWrap: "anywhere" }}>{k}</span>
            <span style={{ color: C_MUTED }}>{v}</span>
          </div>
        ))
      )}
    </div>
  );
};

export const FabricView: React.FC<FabricViewProps> = ({ projectId }) => {
  const status = useFabricData(projectId);
  const [selected, setSelected] = useState<Modality | null>(null);

  useEffect(() => setSelected(null), [projectId]);

  const model = useMemo(() => {
    if (status.kind !== "ready") return null;
    const { graphs, similar } = status.data;
    const perModality = MODALITIES.map((m) => {
      const gs = graphs.filter((g) => g.modality === m);
      return {
        modality: m,
        graphs: gs,
        nodes: gs.reduce((n, g) => n + g.nodeCount, 0),
        edges: gs.reduce((n, g) => n + g.edgeCount, 0),
        nodeTypes: gs.reduce<Record<string, number>>((acc, g) => {
          for (const [k, v] of Object.entries(g.nodeTypes)) acc[k] = (acc[k] ?? 0) + v;
          return acc;
        }, {}),
        edgeTypes: gs.reduce<Record<string, number>>((acc, g) => {
          for (const [k, v] of Object.entries(g.edgeTypes)) acc[k] = (acc[k] ?? 0) + v;
          return acc;
        }, {}),
        crossModalEdges: gs.reduce((n, g) => n + g.crossModalEdges, 0),
      };
    });
    const maxNodes = Math.max(0, ...perModality.map((p) => p.nodes));

    // Cross-modal cluster links: SimilarTo between graph containers of different modalities.
    const pairKey = (a: Modality, b: Modality) => [a, b].sort().join("|");
    const links = new Map<string, { a: Modality; b: Modality; count: number }>();
    let sameModality = 0;
    let external = 0;
    for (const s of similar) {
      if (s.toModality === null) external += 1;
      else if (s.toModality === s.fromModality) sameModality += 1;
      else {
        const k = pairKey(s.fromModality, s.toModality);
        const cur = links.get(k) ?? { a: s.fromModality, b: s.toModality, count: 0 };
        cur.count += 1;
        links.set(k, cur);
      }
    }
    return { perModality, maxNodes, links: Array.from(links.values()), sameModality, external, similarTotal: similar.length };
  }, [status]);

  if (projectId === null) {
    return <div style={{ padding: 16, color: C_MUTED, fontSize: 12.5 }}>Select a project to see its modality fabric.</div>;
  }
  if (status.kind === "idle" || status.kind === "loading") {
    return <div style={{ padding: 16, color: C_MUTED, fontSize: 12.5 }}>Loading fabric…</div>;
  }
  if (status.kind === "error") {
    return <div style={{ padding: 16, color: "#ff8a8a", fontSize: 12.5 }}>Error: {status.message}</div>;
  }
  if (status.kind === "empty" || !model) {
    return (
      <div style={{ padding: 16 }}>
        <Note>
          Project {projectId} has no persisted modality graphs (no ModalityGraph containers parented to it) — nothing
          fabricated to show in their place.
        </Note>
      </div>
    );
  }

  const { data } = status;
  const selectedCluster = model.perModality.find((p) => p.modality === selected) ?? null;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div style={{ display: "flex", gap: 12, flexWrap: "wrap", alignItems: "flex-start" }}>
        <div
          style={{
            flex: "1 1 460px",
            minWidth: 320,
            border: `1px solid ${C_BORDER}`,
            borderRadius: 10,
            background: C_PANEL,
            overflow: "hidden",
          }}
        >
          <svg viewBox={`0 0 ${W} ${H}`} width="100%" style={{ display: "block", maxHeight: 420 }} role="img" aria-label="Modality fabric">
            {model.links.map((l) => {
              const a = CENTERS[l.a];
              const b = CENTERS[l.b];
              return (
                <g key={`${l.a}-${l.b}`}>
                  <line x1={a.x} y1={a.y} x2={b.x} y2={b.y} stroke="#c58bff" strokeWidth={1 + Math.min(l.count, 8)} opacity={0.75} />
                  <text x={(a.x + b.x) / 2} y={(a.y + b.y) / 2 - 6} fontSize={11} fill="#c58bff" textAnchor="middle">
                    {l.count} SimilarTo
                  </text>
                </g>
              );
            })}
            {model.perModality.map((p) => {
              const c = CENTERS[p.modality];
              const empty = p.nodes === 0;
              const r = clusterRadius(p.nodes, model.maxNodes);
              const isSel = selected === p.modality;
              return (
                <g key={p.modality} style={{ cursor: "pointer" }} onClick={() => setSelected(isSel ? null : p.modality)}>
                  <circle
                    cx={c.x}
                    cy={c.y}
                    r={r}
                    fill={MODALITY_COLOR[p.modality]}
                    fillOpacity={empty ? 0.06 : 0.22}
                    stroke={MODALITY_COLOR[p.modality]}
                    strokeWidth={isSel ? 3 : 1.5}
                    strokeDasharray={empty ? "4 3" : undefined}
                  />
                  <text x={c.x} y={c.y - 2} textAnchor="middle" fontSize={13} fontWeight={700} fill={C_TEXT} style={{ pointerEvents: "none" }}>
                    {p.modality}
                  </text>
                  <text x={c.x} y={c.y + 14} textAnchor="middle" fontSize={11} fill={C_MUTED} style={{ pointerEvents: "none" }}>
                    {empty ? (p.graphs.length ? "graph unreadable / empty" : "no graph") : `${p.nodes} nodes · ${p.edges} edges`}
                  </text>
                </g>
              );
            })}
          </svg>
        </div>

        <div style={{ flex: "0 1 260px", minWidth: 220, border: `1px solid ${C_BORDER}`, borderRadius: 10, padding: 12 }}>
          <div style={{ fontSize: 12.5, fontWeight: 700, color: C_TEXT, marginBottom: 8 }}>
            {selectedCluster ? `${selectedCluster.modality} cluster` : "Cluster detail"}
          </div>
          {!selectedCluster && <div style={{ fontSize: 12, color: C_MUTED }}>Click a cluster to see its real type breakdown.</div>}
          {selectedCluster && (
            <>
              <div style={{ fontSize: 12, color: C_BODY, marginBottom: 8 }}>
                {selectedCluster.graphs.length} graph container{selectedCluster.graphs.length === 1 ? "" : "s"} ·{" "}
                {selectedCluster.nodes} nodes · {selectedCluster.edges} edges
                {selectedCluster.crossModalEdges > 0 && ` · ${selectedCluster.crossModalEdges} in-graph cross-modal edges`}
              </div>
              <Breakdown title="Node types" rec={selectedCluster.nodeTypes} />
              <Breakdown title="Edge types" rec={selectedCluster.edgeTypes} />
            </>
          )}
          <button
            onClick={() => navigateTo({ kind: "tab", tabId: "graph-view" })}
            style={{
              marginTop: 6,
              background: "transparent",
              color: C_BODY,
              border: `1px solid ${C_BORDER}`,
              borderRadius: 6,
              padding: "4px 10px",
              fontSize: 12,
              cursor: "pointer",
            }}
          >
            Open in Graph View
          </button>
        </div>
      </div>

      {model.links.length === 0 && (
        <Note>
          No cross-modal relations recorded for this project — no links are drawn between clusters.
          {model.similarTotal > 0 &&
            ` (${model.similarTotal} SimilarTo relation${model.similarTotal === 1 ? "" : "s"} exist but none join graph containers of different modalities in this project: ${model.sameModality} same-modality, ${model.external} to containers outside it.)`}
        </Note>
      )}
      {model.links.length > 0 && (model.sameModality > 0 || model.external > 0) && (
        <Note>
          Also recorded but not drawn as cluster links: {model.sameModality} same-modality SimilarTo, {model.external} to
          containers outside this project.
        </Note>
      )}

      <div style={{ border: `1px solid ${C_BORDER}`, borderRadius: 10, padding: 12, overflowX: "auto" }}>
        <div style={{ fontSize: 12.5, fontWeight: 700, color: C_TEXT, marginBottom: 6 }}>Graph containers ({data.graphs.length})</div>
        <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12, color: C_BODY }}>
          <thead>
            <tr style={{ color: C_MUTED, textAlign: "left" }}>
              <th style={{ padding: "3px 8px 3px 0" }}>Container</th>
              <th style={{ padding: "3px 8px" }}>Modality</th>
              <th style={{ padding: "3px 8px" }}>Nodes</th>
              <th style={{ padding: "3px 8px" }}>Edges</th>
              <th style={{ padding: "3px 8px" }}>Relations</th>
              <th style={{ padding: "3px 8px" }}>Content</th>
            </tr>
          </thead>
          <tbody>
            {data.graphs.map((g) => (
              <tr key={g.containerId} style={{ borderTop: `1px solid ${C_BORDER}` }}>
                <td style={{ padding: "3px 8px 3px 0" }}>{g.containerId}</td>
                <td style={{ padding: "3px 8px", color: MODALITY_COLOR[g.modality] }}>{g.modality}</td>
                <td style={{ padding: "3px 8px" }}>{g.readable ? g.nodeCount : "—"}</td>
                <td style={{ padding: "3px 8px" }}>{g.readable ? g.edgeCount : "—"}</td>
                <td style={{ padding: "3px 8px" }}>{g.relations.length}</td>
                <td style={{ padding: "3px 8px", color: g.readable ? C_MUTED : C_WARN }}>{g.readable ? "read" : "unreadable"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
};

export default FabricView;
