/**
 * CAD Engine — structural feature-tree view for the Parametric CAD modality
 * (pipeline 120, `assets/pipelines/modalities/CAD/main.rs`, confirmed real
 * and substantive — not a stub).
 *
 * Ground truth, verified directly against the real backend before writing
 * this (not assumed from the operator's pasted spec doc, which describes a
 * fuller B-rep-viewport engine this pass does not attempt):
 *
 * - Real node type (`CADNodeType`, main.rs:~730): CADDocument, PartNode,
 *   AssemblyNode, FeatureNode, SketchNode, SolidBodyNode, plus B-rep
 *   Face/Edge/Vertex node types.
 * - Real per-node fields (`CADGraphNode`, main.rs:783): `part_number`,
 *   `revision`, `nominal_value`/`upper_deviation`/`lower_deviation`/`unit`
 *   (dimensional tolerancing), `feature_type`, `dof`, `is_suppressed`,
 *   `mass_kg`, `material_name`, `bounding_box` — shown only when actually
 *   present, never fabricated.
 * - Real structural edges, confirmed at their actual construction sites
 *   (not from the declared enum, which also lists edge types with zero real
 *   construction sites — `ChildFeature` and `FeatureCreatesBody` are
 *   declared and pattern-matched but never actually pushed anywhere in this
 *   file, confirmed by grep): `Contains` (CADDocument→PartNode, real) and
 *   `ParentFeature` (`from_node=parent, to_node=child`, main.rs:1522 — a
 *   real parametric feature tree, e.g. Extrude→Fillet dependency chains).
 *   This view builds a genuine nested tree from these two, the same
 *   approach as `./ThreeDEngine.tsx`'s revision.
 * - No B-rep solid viewport here — out of scope for one pass, and GetGraph
 *   returns no real face/edge geometry buffer to render anyway, only
 *   structural + tolerancing metadata.
 *
 * Pattern matched against `./CodeCallGraph.tsx`/`./ThreeDEngine.tsx`: same
 * `loadGraphData` fetch, same `GraphViewStatus` states, same inline-style
 * conventions, same honest loading/error/empty handling, same tree-from-
 * real-edges approach.
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
const C_ACCENT = "#ffa726"; // distinct from 3D's accent, matches CAD's warm/industrial tone

type CADRaw = {
  node_type?: string;
  content?: string;
  part_number?: string;
  revision?: string;
  nominal_value?: number;
  upper_deviation?: number;
  lower_deviation?: number;
  unit?: string;
  feature_type?: string;
  dof?: number;
  is_suppressed?: boolean;
  mass_kg?: number;
  material_name?: string;
};

function rawCad(node: GraphViewNode): CADRaw {
  return (node.raw ?? {}) as CADRaw;
}

function formatTolerance(r: CADRaw): string | null {
  if (typeof r.nominal_value !== "number") return null;
  const unit = r.unit ?? "";
  const hasDev = typeof r.upper_deviation === "number" || typeof r.lower_deviation === "number";
  if (!hasDev) return `${r.nominal_value}${unit}`;
  const up = typeof r.upper_deviation === "number" ? `+${r.upper_deviation}` : "+0";
  const lo = typeof r.lower_deviation === "number" ? r.lower_deviation.toString() : "0";
  return `${r.nominal_value}${unit} (${up}/${lo})`;
}

// Real, construction-verified structural edge types for CAD (see header
// comment) — `ChildFeature`/`FeatureCreatesBody` are declared but have no
// real construction site anywhere in the backend, so they're excluded here
// rather than silently treated as if they carried real data.
const STRUCTURAL_EDGE_TYPES = new Set(["Contains", "ParentFeature"]);

interface FeatureTreeNode {
  node: GraphViewNode;
  children: FeatureTreeNode[];
}

function buildFeatureTree(nodes: GraphViewNode[], edges: GraphViewEdge[]): { roots: FeatureTreeNode[]; ungrouped: GraphViewNode[] } {
  const childIdsOf = new Map<string, string[]>();
  const hasParent = new Set<string>();
  for (const e of edges) {
    if (!STRUCTURAL_EDGE_TYPES.has(e.edgeType)) continue;
    if (!childIdsOf.has(e.from)) childIdsOf.set(e.from, []);
    childIdsOf.get(e.from)!.push(e.to);
    hasParent.add(e.to);
  }
  const byId = new Map(nodes.map((n) => [n.id, n]));

  function build(id: string, seen: Set<string>): FeatureTreeNode | null {
    const node = byId.get(id);
    if (!node || seen.has(id)) return null;
    const nextSeen = new Set(seen).add(id);
    const children = (childIdsOf.get(id) ?? [])
      .map((cid) => build(cid, nextSeen))
      .filter((c): c is FeatureTreeNode => c !== null)
      .sort((a, b) => a.node.label.localeCompare(b.node.label));
    return { node, children };
  }

  const roots = nodes
    .filter((n) => !hasParent.has(n.id))
    .map((n) => build(n.id, new Set()))
    .filter((t): t is FeatureTreeNode => t !== null)
    .sort((a, b) => a.node.label.localeCompare(b.node.label));

  const visited = new Set<string>();
  const mark = (t: FeatureTreeNode) => {
    visited.add(t.node.id);
    t.children.forEach(mark);
  };
  roots.forEach(mark);
  const ungrouped = nodes.filter((n) => !visited.has(n.id));

  return { roots, ungrouped };
}

const TreeRow: React.FC<{
  entry: FeatureTreeNode;
  depth: number;
  selectedId: string | null;
  onSelect: (id: string) => void;
  expanded: Set<string>;
  toggle: (id: string) => void;
}> = ({ entry, depth, selectedId, onSelect, expanded, toggle }) => {
  const r = rawCad(entry.node);
  const isOpen = expanded.has(entry.node.id);
  const hasChildren = entry.children.length > 0;
  const tol = formatTolerance(r);
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
          background: entry.node.id === selectedId ? "rgba(255,167,38,0.12)" : "transparent",
          opacity: r.is_suppressed ? 0.5 : 1,
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
        {r.is_suppressed && <span style={{ color: C_MUTED, fontSize: 10.5, fontStyle: "italic" }}>suppressed</span>}
        {tol && <span style={{ color: C_MUTED, fontSize: 11 }}>{tol}</span>}
        {r.part_number && <span style={{ color: C_MUTED, fontSize: 11 }}>{r.part_number}</span>}
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
  const r = rawCad(node);
  const fields: [string, string | null][] = [
    ["Type", r.node_type ?? node.nodeType],
    ["Content", r.content ?? null],
    ["Part number", r.part_number ?? null],
    ["Revision", r.revision ?? null],
    ["Tolerance", formatTolerance(r)],
    ["Feature type", r.feature_type ?? null],
    ["DOF", typeof r.dof === "number" ? String(r.dof) : null],
    ["Suppressed", typeof r.is_suppressed === "boolean" ? (r.is_suppressed ? "yes" : "no") : null],
    ["Material", r.material_name ?? null],
    ["Mass", typeof r.mass_kg === "number" ? `${r.mass_kg} kg` : null],
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

export const CADEngine: React.FC<{ projectId: number | null }> = ({ projectId }) => {
  const [status, setStatus] = useState<GraphViewStatus>({ kind: "loading" });
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());

  useEffect(() => {
    if (projectId === null) return;
    setStatus({ kind: "loading" });
    setSelectedId(null);
    const dispose = loadGraphData(projectId, setStatus, { modality: "CAD" });
    return dispose;
  }, [projectId]);

  const tree = useMemo(() => {
    if (status.kind !== "ready") return null;
    return buildFeatureTree(status.data.nodes, status.data.edges);
  }, [status]);

  useEffect(() => {
    if (!tree) return;
    const withChildren = new Set<string>();
    const walk = (t: FeatureTreeNode) => {
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

  const selectedNode = useMemo(
    () => (status.kind === "ready" ? status.data.nodes.find((n) => n.id === selectedId) ?? null : null),
    [status, selectedId],
  );

  if (projectId === null) {
    return <div style={{ color: C_MUTED, fontSize: 12.5 }}>Select a project to see its CAD model.</div>;
  }

  return (
    <div>
      <div style={{ fontSize: 12.5, fontWeight: 700, color: C_TEXT, marginBottom: 8 }}>CAD Model</div>

      {status.kind === "loading" && <div style={{ color: C_MUTED, fontSize: 12.5 }}>Loading CAD graph…</div>}
      {status.kind === "error" && (
        <div style={{ color: C_ERROR, fontSize: 12.5 }}>Error loading CAD graph: {status.message}</div>
      )}
      {status.kind === "empty" && (
        <div style={{ color: C_MUTED, fontSize: 12.5 }}>No CAD-modality graph exists for this project yet — nothing to show.</div>
      )}

      {status.kind === "ready" && tree && (
        <div style={{ display: "flex", gap: 14 }}>
          <div style={{ flex: "1 1 55%", display: "flex", flexDirection: "column", gap: 10, minWidth: 0 }}>
            <div style={{ border: `1px solid ${C_BORDER}`, borderRadius: 8, padding: "6px 8px" }}>
              <div style={{ color: C_MUTED, fontSize: 10.5, textTransform: "uppercase", letterSpacing: 0.4, marginBottom: 4 }}>
                Feature tree ({tree.roots.length} root{tree.roots.length === 1 ? "" : "s"})
              </div>
              {tree.roots.length === 0 ? (
                <div style={{ color: C_MUTED, fontSize: 11.5 }}>No structural edges found among this project's CAD nodes.</div>
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
                  Ungrouped ({tree.ungrouped.length}) — no structural edge connects these to a document root
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

export default CADEngine;
