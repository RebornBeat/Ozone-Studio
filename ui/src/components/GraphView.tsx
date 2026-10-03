/**
 * GraphView — Context Viewer: Graph View canvas shell (Batch C, fork C1).
 *
 * Generic across modalities: fetches real ModalityGraph data (B1) for a
 * chosen project via `loadGraphData` (graphViewData.ts) and draws it with
 * zoom/pan/select/hover. Node/edge VISUAL rendering dispatches through
 * `nodeVisual`/`edgeVisual` (graphRenderers/index.ts) to the real
 * per-modality renderers (code/math/text/image — C2-C7 plus the later image
 * addition), each keyed off construction-verified node/edge type registries
 * per modality; a not-real/schema-only type is drawn dashed/dimmed, never
 * hidden or styled as if confirmed. No graph-layout library is in this
 * project's dependencies (checked ui/package.json) — node positions below
 * are a synthetic circular layout for display purposes only, not a claim
 * about any real backend geometry; the node/edge DATA itself is 100% real,
 * fetched live, never fabricated.
 *
 * No project-selection state exists anywhere else in this app (checked
 * services/store.ts) — this panel includes its own minimal real workspace
 * → project picker (B12, confirmed already servable) so it has an id to
 * fetch against.
 */
import React, { useEffect, useMemo, useRef, useState } from "react";
import { zseiQuery } from "../ozoneClient";
import { loadGraphData } from "../graphViewData";
import { EdgeClass, GraphViewEdge, GraphViewNode, GraphViewStatus } from "../graphViewTypes";
import { edgeVisual, nodeVisual } from "../graphRenderers";
import { GRAPH_OVERLAYS } from "../graphRenderers/overlays";
import { NodeShapeSvg } from "../graphRenderers/NodeShapeSvg";
import NodeDetailPanel from "./NodeDetailPanel";
import EdgeLegendFilters, { edgeTypeKey, modalityOfEdge } from "./EdgeLegendFilters";
import EdgeProvenance from "./EdgeProvenance";
import { onNavigate, takePendingNavigation } from "../navigation";

/** Walk parent_id up from any container until a real Project container is
 * found (or the chain ends) — J2's real resolution for a bare `container`
 * deep link, which carries no project id of its own. */
async function resolveOwningProject(containerId: number): Promise<number | null> {
  let current = containerId;
  for (let hop = 0; hop < 12; hop++) {
    const c = (await zseiQuery<any>({ GetContainer: { container_id: current } }))?.Container;
    if (!c) return null;
    if (c.local_state?.metadata?.container_type === "Project") return current;
    const parent = c.global_state?.parent_id;
    if (!parent || parent === current) return null;
    current = parent;
  }
  return null;
}

interface ProjectOption {
  id: number;
  name: string;
  workspaceId: number;
  workspaceName: string;
}

function currentUserId(): number {
  // Same real fallback pattern already used in services/store.ts.
  return (window as any).ozone?.auth?.getCurrentUserId?.() ?? 1;
}

async function loadProjectOptions(): Promise<ProjectOption[]> {
  const wsResult = await zseiQuery<any>({ GetUserWorkspaces: { user_id: currentUserId() } });
  const workspaceIds: number[] = wsResult?.Containers ?? [];
  const options: ProjectOption[] = [];
  for (const wsId of workspaceIds) {
    const wsContainer = await zseiQuery<any>({ GetContainer: { container_id: wsId } });
    const ws = wsContainer?.Container;
    if (!ws) continue;
    const wsName: string = ws.local_state?.metadata?.name ?? `Workspace ${wsId}`;
    for (const projectId of ws.global_state?.child_ids ?? []) {
      const projContainer = await zseiQuery<any>({ GetContainer: { container_id: projectId } });
      const proj = projContainer?.Container;
      if (!proj || proj.local_state?.metadata?.container_type !== "Project") continue;
      options.push({
        id: projectId,
        name: proj.local_state?.metadata?.name ?? `Project ${projectId}`,
        workspaceId: wsId,
        workspaceName: wsName,
      });
    }
  }
  return options;
}

// ── Synthetic display-only layout (see file header) ──────────────────────

interface LayoutNode extends GraphViewNode {
  x: number;
  y: number;
}

const ARROW_DIRECTIONS: Record<string, readonly [number, number]> = {
  ArrowUp: [0, -1],
  ArrowDown: [0, 1],
  ArrowLeft: [-1, 0],
  ArrowRight: [1, 0],
};

/** Nearest node strictly in the given direction from `from`, scored by real
 * distance plus a lateral-deviation penalty (favors nodes roughly "in line"
 * with the pure axis over ones merely on the correct side). Null when
 * nothing lies in that direction. */
function nearestNodeInDirection(
  from: LayoutNode,
  dir: readonly [number, number],
  candidates: LayoutNode[],
): LayoutNode | null {
  let best: LayoutNode | null = null;
  let bestScore = Infinity;
  for (const c of candidates) {
    if (c.id === from.id) continue;
    const dx = c.x - from.x;
    const dy = c.y - from.y;
    const dot = dx * dir[0] + dy * dir[1];
    if (dot <= 0) continue;
    const dist = Math.sqrt(dx * dx + dy * dy);
    const lateral = Math.abs(dx * dir[1] - dy * dir[0]);
    const score = dist + lateral * 1.5;
    if (score < bestScore) {
      bestScore = score;
      best = c;
    }
  }
  return best;
}

function layoutNodes(nodes: GraphViewNode[]): LayoutNode[] {
  const n = nodes.length;
  if (n === 0) return [];
  // Concentric rings grouped by modality — enough to distinguish clusters
  // without needing a real force-directed layout library (none installed).
  const byModality = new Map<string, GraphViewNode[]>();
  for (const node of nodes) {
    const list = byModality.get(node.modality) ?? [];
    list.push(node);
    byModality.set(node.modality, list);
  }
  const modalities = Array.from(byModality.keys());
  const ringGap = 220;
  const out: LayoutNode[] = [];
  modalities.forEach((modality, ringIndex) => {
    const list = byModality.get(modality)!;
    const radius = 100 + ringIndex * ringGap;
    list.forEach((node, i) => {
      const angle = (2 * Math.PI * i) / Math.max(list.length, 1);
      out.push({
        ...node,
        x: radius * Math.cos(angle),
        y: radius * Math.sin(angle),
      });
    });
  });
  return out;
}

export const GraphView: React.FC = () => {
  const [projects, setProjects] = useState<ProjectOption[]>([]);
  const [projectsError, setProjectsError] = useState<string | null>(null);
  const [selectedProjectId, setSelectedProjectId] = useState<number | null>(null);
  const [status, setStatus] = useState<GraphViewStatus>({ kind: "loading" });

  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [hoveredNodeId, setHoveredNodeId] = useState<string | null>(null);
  const [selectedEdgeId, setSelectedEdgeId] = useState<string | null>(null);
  const [hoveredEdgeId, setHoveredEdgeId] = useState<string | null>(null);
  const [hiddenEdgeClasses, setHiddenEdgeClasses] = useState<Set<EdgeClass>>(new Set());
  const [hiddenEdgeTypes, setHiddenEdgeTypes] = useState<Set<string>>(new Set());
  const [enabledOverlays, setEnabledOverlays] = useState<Set<string>>(new Set());
  const [overlayResults, setOverlayResults] = useState<
    Record<string, { status: "loading" | "ready" | "error"; nodes: GraphViewNode[]; edges: GraphViewEdge[]; message?: string }>
  >({});

  function toggleInSet<T>(setter: React.Dispatch<React.SetStateAction<Set<T>>>, value: T) {
    setter((prev) => {
      const next = new Set(prev);
      if (next.has(value)) next.delete(value);
      else next.add(value);
      return next;
    });
  }

  // Pan/zoom state — plain CSS transform on an SVG <g>, no library needed.
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const dragState = useRef<{ startX: number; startY: number; panX: number; panY: number } | null>(null);

  useEffect(() => {
    let cancelled = false;
    loadProjectOptions()
      .then((opts) => {
        if (cancelled) return;
        setProjects(opts);
        if (opts.length > 0) setSelectedProjectId(opts[0].id);
      })
      .catch((err) => {
        if (!cancelled) setProjectsError(err instanceof Error ? err.message : String(err));
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (selectedProjectId === null) return;
    setSelectedNodeId(null);
    setZoom(1);
    setPan({ x: 0, y: 0 });
    const dispose = loadGraphData(selectedProjectId, setStatus);
    return dispose;
  }, [selectedProjectId]);

  // J2 cross-view navigation: a `graph-node` target names its own project, a
  // bare `container` target doesn't (resolved via parent-walk). Either way,
  // the actual node selection can only happen once that project's real graph
  // data has finished loading — held in `pendingNodeId` until `statusRef`
  // (kept current via the effect below, since the nav effect has empty deps
  // and would otherwise see a stale `status` closure) says ready.
  const pendingNodeId = useRef<string | null>(null);
  const statusRef = useRef(status);
  useEffect(() => {
    statusRef.current = status;
    if (status.kind === "ready" && pendingNodeId.current) {
      if (status.data.nodes.some((n) => n.id === pendingNodeId.current)) {
        setSelectedNodeId(pendingNodeId.current);
      }
      pendingNodeId.current = null;
    }
  }, [status]);

  useEffect(() => {
    function selectInProject(projectId: number, nodeId: string) {
      pendingNodeId.current = nodeId;
      setSelectedProjectId((prev) => {
        if (prev === projectId) {
          // Already on this project — status won't change, so the `status`
          // effect above won't fire again to apply the pending id. Apply now
          // if the data we already have is ready.
          const s = statusRef.current;
          if (s.kind === "ready" && s.data.nodes.some((n) => n.id === nodeId)) {
            setSelectedNodeId(nodeId);
            pendingNodeId.current = null;
          }
          return prev;
        }
        return projectId;
      });
    }

    // Pick up a target that arrived before this component mounted.
    const initial = takePendingNavigation("graph-node");
    if (initial && initial.kind === "graph-node") selectInProject(initial.projectId, initial.nodeId);
    const initialContainer = takePendingNavigation("container");
    if (initialContainer && initialContainer.kind === "container") {
      resolveOwningProject(initialContainer.containerId).then((pid) => {
        if (pid !== null) setSelectedProjectId((prev) => (prev === pid ? prev : pid));
      });
    }

    return onNavigate((target) => {
      if (target.kind === "graph-node") {
        selectInProject(target.projectId, target.nodeId);
      } else if (target.kind === "container") {
        resolveOwningProject(target.containerId).then((pid) => {
          if (pid !== null) setSelectedProjectId((prev) => (prev === pid ? prev : pid));
        });
      }
    });
  }, []);

  useEffect(() => {
    if (status.kind !== "ready" || selectedProjectId === null) return;
    let cancelled = false;
    for (const o of GRAPH_OVERLAYS) {
      if (!enabledOverlays.has(o.id)) continue;
      setOverlayResults((prev) => ({ ...prev, [o.id]: { status: "loading", nodes: [], edges: [] } }));
      o.load({ projectId: selectedProjectId, graphData: status.data })
        .then((r) => {
          if (!cancelled) setOverlayResults((prev) => ({ ...prev, [o.id]: { status: "ready", nodes: r.nodes, edges: r.edges } }));
        })
        .catch((e) => {
          if (!cancelled)
            setOverlayResults((prev) => ({
              ...prev,
              [o.id]: { status: "error", nodes: [], edges: [], message: e instanceof Error ? e.message : String(e) },
            }));
        });
    }
    return () => {
      cancelled = true;
    };
  }, [enabledOverlays, status, selectedProjectId]);

  const mergedNodes = useMemo(() => {
    const byId = new Map<string, GraphViewNode>();
    if (status.kind === "ready") for (const n of status.data.nodes) byId.set(n.id, n);
    for (const o of GRAPH_OVERLAYS) {
      const r = overlayResults[o.id];
      if (enabledOverlays.has(o.id) && r?.status === "ready") for (const n of r.nodes) byId.set(n.id, n);
    }
    return Array.from(byId.values());
  }, [status, overlayResults, enabledOverlays]);
  const mergedEdges = useMemo(() => {
    const byId = new Map<string, GraphViewEdge>();
    if (status.kind === "ready") for (const e of status.data.edges) byId.set(e.id, e);
    for (const o of GRAPH_OVERLAYS) {
      const r = overlayResults[o.id];
      if (enabledOverlays.has(o.id) && r?.status === "ready") for (const e of r.edges) byId.set(e.id, e);
    }
    return Array.from(byId.values());
  }, [status, overlayResults, enabledOverlays]);

  const layoutedNodes = useMemo(() => layoutNodes(mergedNodes), [mergedNodes]);
  const nodeById = useMemo(() => {
    const map = new Map<string, LayoutNode>();
    for (const n of layoutedNodes) map.set(n.id, n);
    return map;
  }, [layoutedNodes]);
  const allEdges: GraphViewEdge[] = mergedEdges;
  const edges = useMemo(
    () =>
      allEdges.filter(
        (e) => !hiddenEdgeClasses.has(e.edgeClass) && !hiddenEdgeTypes.has(edgeTypeKey(modalityOfEdge(e), e.edgeType)),
      ),
    [allEdges, hiddenEdgeClasses, hiddenEdgeTypes],
  );
  const selectedNode = selectedNodeId ? nodeById.get(selectedNodeId) ?? null : null;

  // Keyboard node navigation — roving-focus pattern (listbox-style): the
  // canvas itself is the one tab stop; arrow keys move a lightweight
  // "current" indicator (reuses hoveredNodeId, the same visual the mouse
  // already drives), Enter/Space promotes it to selectedNodeId (opens the
  // detail panel), Escape clears both. Direction search scores candidates
  // by distance plus lateral deviation from the pure axis, so a node
  // roughly "in line" wins over one merely on the correct side.
  const handleCanvasKeyDown = (e: React.KeyboardEvent<SVGSVGElement>) => {
    if (layoutedNodes.length === 0) return;
    const dir = ARROW_DIRECTIONS[e.key];
    if (dir) {
      e.preventDefault();
      const current =
        (hoveredNodeId && nodeById.get(hoveredNodeId)) || (selectedNodeId && nodeById.get(selectedNodeId)) || null;
      const next = current ? nearestNodeInDirection(current, dir, layoutedNodes) : layoutedNodes[0];
      if (next) setHoveredNodeId(next.id);
      return;
    }
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      const target = hoveredNodeId ?? selectedNodeId;
      if (target) setSelectedNodeId((s) => (s === target ? null : target));
      return;
    }
    if (e.key === "Escape") {
      setHoveredNodeId(null);
      setSelectedNodeId(null);
    }
  };
  const provenanceEdge =
    (hoveredEdgeId ? allEdges.find((e) => e.id === hoveredEdgeId) : undefined) ??
    (selectedEdgeId ? allEdges.find((e) => e.id === selectedEdgeId) : undefined) ??
    null;

  function onWheel(e: React.WheelEvent<SVGSVGElement>) {
    e.preventDefault();
    const delta = e.deltaY > 0 ? 0.9 : 1.1;
    setZoom((z) => Math.min(4, Math.max(0.2, z * delta)));
  }
  function onBackgroundMouseDown(e: React.MouseEvent<SVGSVGElement>) {
    dragState.current = { startX: e.clientX, startY: e.clientY, panX: pan.x, panY: pan.y };
  }
  function onBackgroundMouseMove(e: React.MouseEvent<SVGSVGElement>) {
    if (!dragState.current) return;
    const dx = e.clientX - dragState.current.startX;
    const dy = e.clientY - dragState.current.startY;
    setPan({ x: dragState.current.panX + dx, y: dragState.current.panY + dy });
  }
  function endDrag() {
    dragState.current = null;
  }

  return (
    <div className="opanel">
      <div className="opanel-head">
        <span className="opanel-title">Context Viewer — Graph View</span>
      </div>
      <p className="opanel-sub">
        Real node/edge data from each project's persisted modality graphs
        (Code/Math/Text). Generic placeholder shapes here — per-modality
        visual encoding lands in Batch C's C2-C7 forks.
      </p>

      <div style={{ display: "flex", gap: 12, alignItems: "center", marginBottom: 12 }}>
        <label style={{ fontSize: 12.5, color: "var(--color-text-muted)" }}>Project:</label>
        {projectsError && (
          <span style={{ color: "#ff8a8a", fontSize: 12.5 }}>Error loading projects: {projectsError}</span>
        )}
        {!projectsError && projects.length === 0 && (
          <span style={{ color: "var(--color-text-muted)", fontSize: 12.5 }}>No workspaces/projects found yet.</span>
        )}
        {projects.length > 0 && (
          <select
            value={selectedProjectId ?? undefined}
            onChange={(e) => setSelectedProjectId(Number(e.target.value))}
            style={{ background: "#101724", color: "var(--color-text)", border: "1px solid var(--color-border-faint)", borderRadius: 6, padding: "4px 8px" }}
          >
            {projects.map((p) => (
              <option key={p.id} value={p.id}>
                {p.workspaceName} / {p.name}
              </option>
            ))}
          </select>
        )}
      </div>

      <div style={{ display: "flex", gap: 16, alignItems: "center", flexWrap: "wrap", marginBottom: 12, fontSize: 12 }}>
        <span style={{ color: "var(--color-text-muted)" }}>Overlays:</span>
        {GRAPH_OVERLAYS.map((o) => {
          const r = overlayResults[o.id];
          const on = enabledOverlays.has(o.id);
          return (
            <label key={o.id} title={o.description} style={{ display: "flex", alignItems: "center", gap: 5, color: on ? "var(--color-text)" : "var(--color-text-muted)", cursor: "pointer" }}>
              <input type="checkbox" checked={on} style={{ margin: 0 }} onChange={() => toggleInSet<string>(setEnabledOverlays, o.id)} />
              {o.label}
              {on && r?.status === "loading" && <span style={{ color: "var(--color-text-muted)" }}>loading…</span>}
              {on && r?.status === "error" && <span style={{ color: "#ff8a8a" }} title={r.message}>error</span>}
              {on && r?.status === "ready" && <span style={{ color: "var(--color-text-muted)" }}>{r.edges.length} edges</span>}
            </label>
          );
        })}
      </div>

      <div className="opanel-scroll" style={{ display: "flex", gap: 12, minHeight: 0 }}>
        <div style={{ flex: 1, position: "relative", border: "1px solid var(--color-border-faint)", borderRadius: 10, overflow: "hidden", background: "var(--color-bg)" }}>
          {status.kind === "loading" && (
            <div style={{ padding: 24, color: "var(--color-text-muted)" }}>Loading graph…</div>
          )}
          {status.kind === "error" && (
            <div style={{ padding: 24, color: "#ff8a8a" }}>Error: {status.message}</div>
          )}
          {status.kind === "empty" && (
            <div style={{ padding: 24, color: "var(--color-text-muted)" }}>
              No modality graphs exist for this project yet — nothing fabricated to show in their place.
            </div>
          )}
          {status.kind === "ready" && (
            <svg
              width="100%"
              height="100%"
              onWheel={onWheel}
              onMouseDown={onBackgroundMouseDown}
              onMouseMove={onBackgroundMouseMove}
              onMouseUp={endDrag}
              onMouseLeave={endDrag}
              onKeyDown={handleCanvasKeyDown}
              tabIndex={0}
              role="application"
              aria-label={`Graph canvas, ${layoutedNodes.length} nodes. Arrow keys to move between nodes, Enter to open the selected node's detail, Escape to clear selection.${
                hoveredNodeId ? ` Current: ${nodeById.get(hoveredNodeId)?.label ?? hoveredNodeId}.` : ""
              }`}
              style={{ cursor: dragState.current ? "grabbing" : "grab" }}
            >
              <defs>
                <marker id="gv-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
                  <path d="M 0 0 L 10 5 L 0 10 z" fill="#6b7a90" />
                </marker>
              </defs>
              <g transform={`translate(${pan.x + 400} ${pan.y + 300}) scale(${zoom})`}>
                {edges.map((edge) => {
                  const from = nodeById.get(edge.from);
                  const to = nodeById.get(edge.to);
                  if (!from || !to) return null;
                  const highlighted =
                    selectedNodeId === edge.from || selectedNodeId === edge.to || selectedEdgeId === edge.id;
                  const v = edgeVisual(edge, from.modality);
                  return (
                    <g key={edge.id}>
                      <line
                        x1={from.x}
                        y1={from.y}
                        x2={to.x}
                        y2={to.y}
                        stroke={highlighted ? "#e8c14f" : v.stroke}
                        strokeWidth={highlighted ? v.strokeWidth + 1 : v.strokeWidth}
                        strokeDasharray={v.strokeDasharray}
                        opacity={v.opacity ?? 1}
                        markerEnd={v.arrow ? "url(#gv-arrow)" : undefined}
                        style={{ pointerEvents: "none" }}
                      />
                      {/* Wide transparent hit-target so a 1px edge is actually hoverable/clickable. */}
                      <line
                        x1={from.x}
                        y1={from.y}
                        x2={to.x}
                        y2={to.y}
                        stroke="transparent"
                        strokeWidth={10}
                        style={{ cursor: "pointer" }}
                        onMouseEnter={() => setHoveredEdgeId(edge.id)}
                        onMouseLeave={() => setHoveredEdgeId((h) => (h === edge.id ? null : h))}
                        onMouseDown={(e) => e.stopPropagation()}
                        onClick={() => setSelectedEdgeId((s) => (s === edge.id ? null : edge.id))}
                      />
                    </g>
                  );
                })}
                {layoutedNodes.map((node) => {
                  const isSelected = node.id === selectedNodeId;
                  const isHovered = node.id === hoveredNodeId;
                  return (
                    <g
                      key={node.id}
                      transform={`translate(${node.x} ${node.y})`}
                      onClick={() => setSelectedNodeId(node.id === selectedNodeId ? null : node.id)}
                      onMouseEnter={() => setHoveredNodeId(node.id)}
                      onMouseLeave={() => setHoveredNodeId((h) => (h === node.id ? null : h))}
                      role="button"
                      aria-label={`${node.nodeType} node: ${node.label}${isSelected ? " (selected)" : ""}`}
                      style={{ cursor: "pointer" }}
                    >
                      {/* Keyboard-focus ring, distinct from mouse hover — the
                          roving indicator handleCanvasKeyDown drives via
                          hoveredNodeId, made visually distinguishable so a
                          keyboard user can tell "current" from "moused
                          over" even though they share the same state. */}
                      {isHovered && (
                        <circle r={12} fill="none" stroke="var(--color-accent)" strokeWidth={1.5} strokeDasharray="3 2" />
                      )}
                      <NodeShapeSvg visual={nodeVisual(node)} selected={isSelected} hovered={isHovered} />
                      {(isHovered || isSelected) && (
                        <text x={12} y={4} fontSize={11} fill="var(--color-text)">
                          {node.label}
                        </text>
                      )}
                    </g>
                  );
                })}
              </g>
            </svg>
          )}
        </div>

        <div style={{ width: 260, border: "1px solid var(--color-border-faint)", borderRadius: 10, padding: 12, overflowY: "auto" }}>
          <NodeDetailPanel
            node={selectedNode}
            containerRelations={status.kind === "ready" ? status.data.containerRelations : {}}
          />
          <EdgeProvenance edge={provenanceEdge} />
          <EdgeLegendFilters
            edges={allEdges}
            hiddenEdgeClasses={hiddenEdgeClasses}
            hiddenEdgeTypes={hiddenEdgeTypes}
            onToggleClass={(c) => toggleInSet(setHiddenEdgeClasses, c)}
            onToggleType={(t) => toggleInSet(setHiddenEdgeTypes, t)}
          />
          {status.kind === "ready" && (
            <div style={{ marginTop: 16, fontSize: 11, color: "var(--color-text-muted)" }}>
              {status.data.nodes.length} nodes · {status.data.edges.length} edges · sources:{" "}
              {status.data.sourceContainers.join(", ") || "none"}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default GraphView;
