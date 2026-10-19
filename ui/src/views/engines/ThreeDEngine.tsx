/**
 * 3D Engine — structural scene view for the 3D modality (pipeline 109,
 * `assets/pipelines/modalities/3D/main.rs`, 2726 lines — the largest
 * non-text/code modality backend, confirmed real and substantive this
 * session, not a stub).
 *
 * Ground truth, verified directly against the real backend before writing
 * any of this (not assumed from the operator's pasted spec doc, which
 * describes a fuller interactive viewport this pass does not attempt):
 *
 * - Real node type (`ThreeDNodeType`, main.rs:1116): Scene, Collection,
 *   Object, Mesh, Armature, Bone, Material, Light, Camera, RigidBody/
 *   SoftBody/Cloth/FluidDomain/ParticleSystem (physics), PhysicsCache, and
 *   ~30 others.
 * - Real per-node fields (`ThreeDGraphNode`, main.rs:1134): `content`,
 *   `location`, `dimensions`, `physics_type`, `mass`, `material_name`,
 *   `frame_range` — shown only when actually present on a node, never
 *   fabricated.
 * - `RunSimulation` (main.rs:2454) is real in a specific, narrow sense: it
 *   does NOT run an actual physics engine ("In production: dispatch to
 *   physics backend... here we update graph state and return frame count"
 *   — the handler's own comment) — but it DOES persist real evidence: a
 *   `PhysicsCache` node (with `physics_type`/`content` describing the
 *   domain and frame count) connected to the scene root via a `Simulates`
 *   edge. So "has this scene ever had a simulation run" is a real,
 *   answerable question from graph data — shown below as evidence, not
 *   simulated/predicted.
 * - No full interactive WebGL viewport here — out of scope for one pass,
 *   and there is no real backend geometry buffer to render anyway (no mesh
 *   vertex/face data is returned by GetGraph, only structural nodes).
 *   `RunSimulation`/`ExportScene` triggers were deliberately left out too
 *   (this view is read-only, matching CodeCallGraph.tsx's pattern).
 *
 * REAL STRUCTURE, NOT FLAT BUCKETS (2026-10-18, revision): the first pass
 * of this file grouped nodes into 6 coarse type-based buckets, ignoring the
 * real parent/child edges the backend actually constructs. Confirmed
 * directly against main.rs's real edge-construction call sites (not
 * assumed): `Contains` (root→Collection, Object→Mesh, Armature→Bone,
 * Object→Material/Light/Camera — all `from_node=parent, to_node=child`),
 * `ParentOf` (Armature→Bone chains, Object→Object parenting, same
 * direction), `InCollection` (Collection→Object, same direction) are real,
 * genuinely constructed, multi-level structural edges — this view now
 * builds an actual nested tree from them instead of flattening by type.
 * Any node with no incoming structural edge becomes a tree root (handles
 * multiple independent scenes/graphs cleanly); any node unreachable from a
 * root (shouldn't happen given that definition, but guarded regardless)
 * still renders in an "Ungrouped" section rather than being silently
 * dropped — same no-omission doctrine as the rest of this codebase.
 *
 * Pattern matched against `./CodeCallGraph.tsx`: same `loadGraphData`
 * fetch, same `GraphViewStatus` states, same inline-style/CSS-var
 * conventions, same honest loading/error/empty handling.
 */
import React, { useMemo, useState, useEffect } from "react";
import { loadGraphData } from "../../graphViewData";
import { GraphViewNode, GraphViewEdge, GraphViewStatus } from "../../graphViewTypes";
import { navigateTo } from "../../navigation";

const C_TEXT = "var(--color-text)";
const C_BODY = "var(--color-text-secondary)";
const C_MUTED = "var(--color-text-muted)";
const C_BORDER = "var(--color-border-faint)";
const C_PANEL = "var(--color-bg)";
const C_ERROR = "#ff8a8a";
const C_ACCENT = "#4fc3f7"; // MODALITY_COLOR["3D"], defaults.ts

type ThreeDRaw = {
  node_type?: string;
  content?: string;
  location?: [number, number, number];
  dimensions?: [number, number, number];
  physics_type?: string;
  mass?: number;
  material_name?: string;
  frame_range?: [number, number];
};

function raw3d(node: GraphViewNode): ThreeDRaw {
  return (node.raw ?? {}) as ThreeDRaw;
}

function formatVec3(v: [number, number, number] | undefined): string | null {
  if (!v) return null;
  return `(${v.map((n) => n.toFixed(2)).join(", ")})`;
}

// Real, construction-verified structural edge types for 3D (see header
// comment) — the only edge types this tree treats as parent→child.
const STRUCTURAL_EDGE_TYPES = new Set(["Contains", "ParentOf", "InCollection"]);

interface SceneTreeNode {
  node: GraphViewNode;
  children: SceneTreeNode[];
}

function buildSceneTree(nodes: GraphViewNode[], edges: GraphViewEdge[]): { roots: SceneTreeNode[]; ungrouped: GraphViewNode[] } {
  const childIdsOf = new Map<string, string[]>();
  const hasParent = new Set<string>();
  for (const e of edges) {
    if (!STRUCTURAL_EDGE_TYPES.has(e.edgeType)) continue;
    if (!childIdsOf.has(e.from)) childIdsOf.set(e.from, []);
    childIdsOf.get(e.from)!.push(e.to);
    hasParent.add(e.to);
  }
  const byId = new Map(nodes.map((n) => [n.id, n]));

  function build(id: string, seen: Set<string>): SceneTreeNode | null {
    const node = byId.get(id);
    if (!node || seen.has(id)) return null; // guard against a real malformed cycle
    const nextSeen = new Set(seen).add(id);
    const children = (childIdsOf.get(id) ?? [])
      .map((cid) => build(cid, nextSeen))
      .filter((c): c is SceneTreeNode => c !== null)
      .sort((a, b) => a.node.label.localeCompare(b.node.label));
    return { node, children };
  }

  const roots = nodes
    .filter((n) => !hasParent.has(n.id))
    .map((n) => build(n.id, new Set()))
    .filter((t): t is SceneTreeNode => t !== null)
    .sort((a, b) => a.node.label.localeCompare(b.node.label));

  const visited = new Set<string>();
  const mark = (t: SceneTreeNode) => {
    visited.add(t.node.id);
    t.children.forEach(mark);
  };
  roots.forEach(mark);
  const ungrouped = nodes.filter((n) => !visited.has(n.id));

  return { roots, ungrouped };
}

const TreeRow: React.FC<{
  entry: SceneTreeNode;
  depth: number;
  selectedId: string | null;
  onSelect: (id: string) => void;
  expanded: Set<string>;
  toggle: (id: string) => void;
}> = ({ entry, depth, selectedId, onSelect, expanded, toggle }) => {
  const r = raw3d(entry.node);
  const isOpen = expanded.has(entry.node.id);
  const hasChildren = entry.children.length > 0;
  return (
    <div>
      <div
        onClick={() => onSelect(entry.node.id)}
        title="Inspect"
        style={{
          display: "flex",
          alignItems: "baseline",
          gap: 6,
          cursor: "pointer",
          padding: "2px 6px",
          paddingLeft: 6 + depth * 14,
          borderRadius: 4,
          background: entry.node.id === selectedId ? "rgba(79,195,247,0.12)" : "transparent",
        }}
      >
        {hasChildren ? (
          <span
            onClick={(e) => {
              e.stopPropagation();
              toggle(entry.node.id);
            }}
            style={{ color: C_MUTED, fontSize: 10, width: 10, display: "inline-block", cursor: "pointer" }}
          >
            {isOpen ? "▾" : "▸"}
          </span>
        ) : (
          <span style={{ width: 10, display: "inline-block" }} />
        )}
        <span style={{ color: entry.node.id === selectedId ? C_ACCENT : C_BODY, fontSize: 12 }}>
          {r.node_type ?? entry.node.nodeType} — {entry.node.label}
        </span>
        {formatVec3(r.location) && <span style={{ color: C_MUTED, fontSize: 11 }}>{formatVec3(r.location)}</span>}
        {r.physics_type && <span style={{ color: C_MUTED, fontSize: 11 }}>{r.physics_type}</span>}
      </div>
      {hasChildren && isOpen && (
        <div>
          {entry.children.map((child) => (
            <TreeRow
              key={child.node.id}
              entry={child}
              depth={depth + 1}
              selectedId={selectedId}
              onSelect={onSelect}
              expanded={expanded}
              toggle={toggle}
            />
          ))}
        </div>
      )}
    </div>
  );
};

const NodeDetail: React.FC<{ node: GraphViewNode; projectId: number }> = ({ node, projectId }) => {
  const r = raw3d(node);
  const fields: [string, string | null][] = [
    ["Type", r.node_type ?? node.nodeType],
    ["Content", r.content ?? null],
    ["Location", formatVec3(r.location)],
    ["Dimensions", formatVec3(r.dimensions)],
    ["Material", r.material_name ?? null],
    ["Physics type", r.physics_type ?? null],
    ["Mass", typeof r.mass === "number" ? String(r.mass) : null],
    ["Frame range", r.frame_range ? `${r.frame_range[0]} – ${r.frame_range[1]}` : null],
  ];
  return (
    <div style={{ border: `1px solid ${C_BORDER}`, borderRadius: 8, padding: "10px 12px", background: C_PANEL }}>
      <div
        style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline", marginBottom: 6, cursor: "pointer" }}
        onClick={() => navigateTo({ kind: "graph-node", projectId, nodeId: node.id })}
        title="View in Graph"
      >
        <span style={{ color: C_ACCENT, fontWeight: 700, fontSize: 12.5 }}>{node.label}</span>
        <span style={{ color: C_MUTED, fontSize: 10.5 }}>view in graph →</span>
      </div>
      {fields
        .filter(([, v]) => v !== null && v !== "")
        .map(([k, v]) => (
          <div key={k} style={{ display: "flex", gap: 8, fontSize: 11.5, marginTop: 3 }}>
            <span style={{ color: C_MUTED, minWidth: 90 }}>{k}</span>
            <span style={{ color: C_BODY }}>{v}</span>
          </div>
        ))}
    </div>
  );
};

export const ThreeDEngine: React.FC<{ projectId: number | null }> = ({ projectId }) => {
  const [status, setStatus] = useState<GraphViewStatus>({ kind: "loading" });
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());

  useEffect(() => {
    if (projectId === null) return;
    setStatus({ kind: "loading" });
    setSelectedId(null);
    const dispose = loadGraphData(projectId, setStatus, { modality: "3D" });
    return dispose;
  }, [projectId]);

  const tree = useMemo(() => {
    if (status.kind !== "ready") return null;
    return buildSceneTree(status.data.nodes, status.data.edges);
  }, [status]);

  // Default to expanded for every node that has children, once per fresh
  // load — real scenes in this codebase's own sample data are shallow
  // (2-5 nodes), so showing everything open by default reads better than
  // forcing a click per level; the user can still collapse via the toggle.
  useEffect(() => {
    if (!tree) return;
    const withChildren = new Set<string>();
    const walk = (t: SceneTreeNode) => {
      if (t.children.length > 0) withChildren.add(t.node.id);
      t.children.forEach(walk);
    };
    tree.roots.forEach(walk);
    setExpanded(withChildren);
  }, [tree]);

  const toggle = (id: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const physicsCacheNodes = useMemo(
    () => (status.kind === "ready" ? status.data.nodes.filter((n) => (raw3d(n).node_type ?? n.nodeType) === "PhysicsCache") : []),
    [status],
  );

  const selectedNode = useMemo(
    () => (status.kind === "ready" ? status.data.nodes.find((n) => n.id === selectedId) ?? null : null),
    [status, selectedId],
  );

  if (projectId === null) {
    return <div style={{ color: C_MUTED, fontSize: 12.5 }}>Select a project to see its 3D scene.</div>;
  }

  return (
    <div>
      <div style={{ fontSize: 12.5, fontWeight: 700, color: C_TEXT, marginBottom: 8 }}>3D Scene</div>

      {status.kind === "loading" && <div style={{ color: C_MUTED, fontSize: 12.5 }}>Loading 3D graph…</div>}
      {status.kind === "error" && (
        <div style={{ color: C_ERROR, fontSize: 12.5 }}>Error loading 3D graph: {status.message}</div>
      )}
      {status.kind === "empty" && (
        <div style={{ color: C_MUTED, fontSize: 12.5 }}>No 3D-modality graph exists for this project yet — nothing to show.</div>
      )}

      {status.kind === "ready" && tree && (
        <div style={{ display: "flex", gap: 14 }}>
          <div style={{ flex: "1 1 55%", display: "flex", flexDirection: "column", gap: 10, minWidth: 0 }}>
            <div style={{ border: `1px solid ${C_BORDER}`, borderRadius: 8, padding: "6px 8px" }}>
              <div style={{ color: C_MUTED, fontSize: 10.5, textTransform: "uppercase", letterSpacing: 0.4, marginBottom: 4 }}>
                Scene hierarchy ({tree.roots.length} root{tree.roots.length === 1 ? "" : "s"})
              </div>
              {tree.roots.length === 0 ? (
                <div style={{ color: C_MUTED, fontSize: 11.5 }}>No structural edges found among this project's 3D nodes.</div>
              ) : (
                tree.roots.map((root) => (
                  <TreeRow
                    key={root.node.id}
                    entry={root}
                    depth={0}
                    selectedId={selectedId}
                    onSelect={setSelectedId}
                    expanded={expanded}
                    toggle={toggle}
                  />
                ))
              )}
            </div>

            {tree.ungrouped.length > 0 && (
              <div style={{ border: `1px dashed ${C_BORDER}`, borderRadius: 8, padding: "6px 8px" }}>
                <div style={{ color: C_MUTED, fontSize: 10.5, textTransform: "uppercase", letterSpacing: 0.4, marginBottom: 4 }}>
                  Ungrouped ({tree.ungrouped.length}) — no structural edge connects these to a scene root
                </div>
                {tree.ungrouped.map((node) => (
                  <TreeRow
                    key={node.id}
                    entry={{ node, children: [] }}
                    depth={0}
                    selectedId={selectedId}
                    onSelect={setSelectedId}
                    expanded={expanded}
                    toggle={toggle}
                  />
                ))}
              </div>
            )}

            <div style={{ border: `1px dashed ${C_BORDER}`, borderRadius: 8, padding: "8px 10px" }}>
              <div style={{ color: C_MUTED, fontSize: 10.5, textTransform: "uppercase", letterSpacing: 0.4, marginBottom: 4 }}>
                Simulation evidence
              </div>
              {physicsCacheNodes.length === 0 ? (
                <div style={{ color: C_MUTED, fontSize: 11.5, lineHeight: 1.5 }}>
                  No simulation has been run on this scene — `RunSimulation` leaves a real `PhysicsCache` node behind
                  when it runs, and none exists here. (Note: `RunSimulation` updates graph state honestly, it does not
                  drive an actual physics engine — see this file's header comment.)
                </div>
              ) : (
                physicsCacheNodes.map((n) => (
                  <div key={n.id} style={{ fontSize: 11.5, color: C_BODY, marginTop: 2 }}>
                    {raw3d(n).content}
                  </div>
                ))
              )}
            </div>
          </div>

          <div style={{ flex: "1 1 45%", minWidth: 0 }}>
            {selectedNode ? (
              <NodeDetail node={selectedNode} projectId={projectId} />
            ) : (
              <div style={{ color: C_MUTED, fontSize: 12, padding: "8px 2px" }}>Select a node to inspect it.</div>
            )}
          </div>
        </div>
      )}
    </div>
  );
};

export default ThreeDEngine;
