/**
 * G1 — Code Intelligence Engine shell.
 *
 * A COMPOSITION over real, already-landed pieces — nothing here re-implements
 * data fetching or rendering that exists elsewhere:
 *   - File tree:   ../../data/workspaceData `loadProjectFileRefs(projectId)`
 *                  (F1, real `FileReference` containers), filtered to
 *                  `kind==="file" && modality==="code"` (the real per-file
 *                  modality lives in `FileRef.modality`, set from the file's
 *                  own graph keywords — see workspaceData.ts's header).
 *   - Code view:   ./CodeViewer (F3, real tokenizer + gutter graph markers),
 *                  reading whatever `../../fileSelection` currently holds.
 *   - Graph panel: ../../graphViewData `loadGraphData(projectId, cb,
 *                  {modality:"code"})` (B1/C1), drawn with the SAME real
 *                  per-modality visuals as the full Graph View
 *                  (../../graphRenderers `nodeVisual`/`edgeVisual`,
 *                  ../../graphRenderers/NodeShapeSvg) — a smaller, code-only
 *                  canvas, not a re-implementation of the general multi-
 *                  modality view (no zoom/pan/legend here; those live in the
 *                  Graph View tab).
 *
 * Cross-pane wiring, both directions, using pieces that already exist:
 *   - Click a file in the tree      -> `setSelectedFile` (../../fileSelection).
 *   - Click a function/class node   -> `navigateTo({kind:"code-file",...})`
 *     in the graph panel               (../../navigation) using the node's
 *                                       real `CodePosition` (`raw.position`)
 *                                       or, for a whole-file `File` node,
 *                                       `raw.name` (= the analysed file_path).
 *                                       CodeViewer already listens for this
 *                                       exact target (confirmed by reading
 *                                       it: `path = navTarget?.path ??
 *                                       selectedFile?.path`), so no edit to
 *                                       CodeViewer is needed for this
 *                                       direction.
 *   - Click a gutter marker in the
 *     code view for the reverse
 *     direction                     -> already wired inside CodeViewer/F4
 *                                       (`navigateTo({kind:"graph-node"})`);
 *                                       this shell just listens for it to
 *                                       highlight the matching graph node.
 *
 * Real current state (verified live against the running host, 2026-09-27):
 * the one real project (1082, workspace 1081) has ZERO children — no linked
 * files, no persisted code graph. Every empty state below is the REAL
 * primary case right now, not a hypothetical edge case, and is designed
 * accordingly (a plain "nothing here yet" message, not a fabricated sample).
 */
import React, { useEffect, useMemo, useRef, useState } from "react";
import { FileRef, loadProjectFileRefs } from "../../data/workspaceData";
import { SelectedFile, setSelectedFile, useSelectedFile } from "../../fileSelection";
import { loadGraphData } from "../../graphViewData";
import { GraphViewData, GraphViewEdge, GraphViewNode, GraphViewStatus } from "../../graphViewTypes";
import { edgeVisual, nodeVisual } from "../../graphRenderers";
import { NodeShapeSvg } from "../../graphRenderers/NodeShapeSvg";
import { navigateTo, onNavigate } from "../../navigation";
import CodeViewer from "../files/CodeViewer";

export type CodeEngineProps = { projectId: number | null };

const C_TEXT = "var(--color-text)";
const C_BODY = "var(--color-text-secondary)";
const C_MUTED = "var(--color-text-muted)";
const C_BORDER = "var(--color-border-faint)";
const C_PANEL = "var(--color-bg)";

const wrap: React.CSSProperties = { overflowWrap: "anywhere", wordBreak: "break-word", minWidth: 0 };

// ── File tree (left pane) ───────────────────────────────────────────────

const FileTree: React.FC<{ projectId: number | null }> = ({ projectId }) => {
  const [refs, setRefs] = useState<FileRef[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selected] = useSelectedFile();

  useEffect(() => {
    if (projectId === null) {
      setRefs(null);
      return;
    }
    let cancelled = false;
    setRefs(null);
    setError(null);
    loadProjectFileRefs(projectId)
      .then((all) => {
        if (!cancelled) setRefs(all.filter((r) => r.kind === "file" && r.modality === "code"));
      })
      .catch((e) => !cancelled && setError(e instanceof Error ? e.message : String(e)));
    return () => {
      cancelled = true;
    };
  }, [projectId]);

  if (projectId === null) return <div style={{ padding: 10, color: C_MUTED, fontSize: 12 }}>No project selected.</div>;
  if (error) return <div style={{ padding: 10, color: "#ff8a8a", fontSize: 12, ...wrap }}>Error loading files: {error}</div>;
  if (refs === null) return <div style={{ padding: 10, color: C_MUTED, fontSize: 12 }}>Loading files…</div>;
  if (refs.length === 0) {
    return (
      <div style={{ padding: 10, color: C_MUTED, fontSize: 12, lineHeight: 1.6 }}>
        No code files linked to this project yet — link one via the Files tab.
      </div>
    );
  }
  return (
    <div style={{ padding: 6 }}>
      {refs.map((r) => {
        const isSel = selected?.containerId === r.containerId;
        return (
          <div
            key={r.containerId}
            onClick={() =>
              setSelectedFile({
                projectId: projectId!,
                containerId: r.containerId,
                kind: "file",
                name: r.name,
                path: r.path,
                modality: r.modality,
              } as SelectedFile)
            }
            title={r.path}
            style={{
              padding: "4px 8px",
              borderRadius: 6,
              cursor: "pointer",
              fontSize: 12.5,
              color: isSel ? C_TEXT : C_BODY,
              background: isSel ? "var(--color-border-faint)" : "transparent",
              ...wrap,
            }}
          >
            {r.name}
          </div>
        );
      })}
    </div>
  );
};

// ── Compact code-only graph panel (right pane) ──────────────────────────

interface RawPosition {
  file_path?: unknown;
  start_line?: unknown;
}

/** File path + 1-based line to jump to for a real code node, or null when
 * the node carries nothing navigable (schema-only types, or a position-less
 * node other than the whole-file `File` type). */
function navTargetForNode(node: GraphViewNode): { path: string; line?: number } | null {
  const raw = node.raw as { name?: unknown; node_type?: string; position?: unknown };
  const pos = raw.position as RawPosition | undefined;
  if (pos && typeof pos.file_path === "string" && pos.file_path.length > 0) {
    return { path: pos.file_path, line: typeof pos.start_line === "number" ? pos.start_line : undefined };
  }
  // File nodes carry no position; their path is `name` (see file header).
  if (raw.node_type === "File" && typeof raw.name === "string" && raw.name.length > 0) {
    return { path: raw.name };
  }
  return null;
}

interface LaidOutNode extends GraphViewNode {
  x: number;
  y: number;
}

function layout(nodes: GraphViewNode[]): LaidOutNode[] {
  const n = nodes.length;
  if (n === 0) return [];
  const radius = Math.max(60, 22 * Math.sqrt(n));
  return nodes.map((node, i) => ({
    ...node,
    x: radius * Math.cos((2 * Math.PI * i) / n),
    y: radius * Math.sin((2 * Math.PI * i) / n),
  }));
}

const CodeGraphPanel: React.FC<{ projectId: number | null }> = ({ projectId }) => {
  const [status, setStatus] = useState<GraphViewStatus>({ kind: "loading" });
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);

  useEffect(() => {
    if (projectId === null) {
      setStatus({ kind: "empty" });
      return;
    }
    setSelectedNodeId(null);
    return loadGraphData(projectId, setStatus, { modality: "code" });
  }, [projectId]);

  // Reverse direction: F4's gutter marker click navigates to a graph node —
  // highlight it here when this project's graph is the target.
  useEffect(() => {
    return onNavigate((t) => {
      if (t.kind === "graph-node" && t.projectId === projectId) setSelectedNodeId(t.nodeId);
    });
  }, [projectId]);

  const data: GraphViewData | null = status.kind === "ready" ? status.data : null;
  const nodes = useMemo(() => layout(data?.nodes ?? []), [data]);
  const edges: GraphViewEdge[] = data?.edges ?? [];
  const byId = useMemo(() => new Map(nodes.map((n) => [n.id, n])), [nodes]);
  const selectedNode = selectedNodeId ? byId.get(selectedNodeId) ?? null : null;

  if (status.kind === "loading") return <div style={{ padding: 10, color: C_MUTED, fontSize: 12 }}>Loading code graph…</div>;
  if (status.kind === "error") return <div style={{ padding: 10, color: "#ff8a8a", fontSize: 12, ...wrap }}>Error: {status.message}</div>;
  if (status.kind === "empty" || nodes.length === 0) {
    return <div style={{ padding: 10, color: C_MUTED, fontSize: 12, lineHeight: 1.6 }}>No code graph persisted for this project yet.</div>;
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", minHeight: 0 }}>
      <svg width="100%" height="100%" style={{ flex: 1, minHeight: 0 }}>
        <g transform="translate(140 140)">
          {edges.map((e) => {
            const from = byId.get(e.from);
            const to = byId.get(e.to);
            if (!from || !to) return null;
            const v = edgeVisual(e, "code");
            const highlighted = selectedNodeId === e.from || selectedNodeId === e.to;
            return (
              <line
                key={e.id}
                x1={from.x}
                y1={from.y}
                x2={to.x}
                y2={to.y}
                stroke={highlighted ? "#e8c14f" : v.stroke}
                strokeWidth={highlighted ? v.strokeWidth + 1 : v.strokeWidth}
                strokeDasharray={v.strokeDasharray}
                opacity={v.opacity ?? 1}
              />
            );
          })}
          {nodes.map((node) => {
            const target = navTargetForNode(node);
            return (
              <g
                key={node.id}
                transform={`translate(${node.x} ${node.y})`}
                onClick={() => {
                  setSelectedNodeId(node.id);
                  if (projectId !== null && target) navigateTo({ kind: "code-file", projectId, path: target.path, line: target.line });
                }}
                style={{ cursor: target ? "pointer" : "default" }}
              >
                <NodeShapeSvg visual={nodeVisual(node)} selected={node.id === selectedNodeId} hovered={false} />
              </g>
            );
          })}
        </g>
      </svg>
      <div style={{ borderTop: `1px solid ${C_BORDER}`, padding: "6px 10px", fontSize: 11, color: C_MUTED }}>
        {nodes.length} nodes · {edges.length} edges
        {selectedNode && (
          <span style={{ marginLeft: 10, color: C_BODY }}>
            selected: <b>{selectedNode.label}</b> ({selectedNode.nodeType}
            {!selectedNode.isReal && ", schema-only"})
          </span>
        )}
      </div>
    </div>
  );
};

// ── Shell ────────────────────────────────────────────────────────────────

export const CodeEngine: React.FC<CodeEngineProps> = ({ projectId }) => {
  const paneStyle: React.CSSProperties = {
    border: `1px solid ${C_BORDER}`,
    borderRadius: 10,
    background: C_PANEL,
    overflow: "hidden",
    display: "flex",
    flexDirection: "column",
    minHeight: 0,
  };
  return (
    <div style={{ display: "flex", gap: 10, height: "100%", minHeight: 0 }}>
      <div style={{ ...paneStyle, width: 220, overflowY: "auto" }}>
        <PaneHeader>Files</PaneHeader>
        <FileTree projectId={projectId} />
      </div>
      <div style={{ ...paneStyle, flex: 2, minWidth: 0 }}>
        <CodeViewer projectId={projectId} />
      </div>
      <div style={{ ...paneStyle, flex: 1, minWidth: 260 }}>
        <PaneHeader>Call graph</PaneHeader>
        <CodeGraphPanel projectId={projectId} />
      </div>
    </div>
  );
};

const PaneHeader: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <div
    style={{
      padding: "6px 10px",
      fontSize: 11,
      fontWeight: 700,
      letterSpacing: 0.4,
      textTransform: "uppercase",
      color: C_MUTED,
      borderBottom: `1px solid ${C_BORDER}`,
    }}
  >
    {children}
  </div>
);

export default CodeEngine;
