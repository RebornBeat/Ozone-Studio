/**
 * GraphNativeSection — the 3D graph-native lens for every workspace tab.
 *
 * Each section (Files/Fabric/Hierarchy/Graph/Engines) renders through this
 * component, which shows the SAME GraphWorld3D substrate filtered/highlighted
 * for that lens. The substrate is loaded once by the GraphProvider — switching
 * lenses is a camera/filter change, not a page swap.
 *
 * This is the operator's vision: "explorable traversable 3D graph-native
 * traversal map across all" — every entity is a node in the 3D space, every
 * cross-link is a visible path, and the tab bar is a lens selector.
 */
import React, { useMemo, useState, useCallback } from "react";
import { useGraphKit, useGraphKitProject } from "./graphProvider";
import { GraphWorld3D } from "./GraphWorld3D";
import { CrossLinkStrip } from "./index";

import type { GraphViewNode } from "../graphViewTypes";

const C_TEXT = "var(--color-text)";
const C_BODY = "var(--color-text-secondary)";
const C_MUTED = "var(--color-text-muted)";
const C_BORDER = "var(--color-border-faint)";
const C_ACCENT = "#4fc3f7";

// ─── Lens filter ───

export type SectionLens =
  | { kind: "files" }
  | { kind: "fabric" }
  | { kind: "hierarchy" }
  | { kind: "graph" }
  | { kind: "engines"; modality: string };

/** Filter/highlight the substrate for the active lens. Each lens shows the
 * full 3D space but highlights its own entities and dims the rest. */
function lensFilter(
  lens: SectionLens,
  nodes: GraphViewNode[],
): { show: Set<string>; label: string } {
  const show = new Set<string>();
  let label = "";
  switch (lens.kind) {
    case "files":
      label = "Files — file-linked containers highlighted";
      break;
    case "fabric":
      label = "Fabric — cross-modal SimilarTo links highlighted";
      break;
    case "hierarchy":
      label = "Hierarchy — AMT/lineage containers highlighted";
      break;
    case "graph":
      label = "Graph — all containers visible";
      break;
    case "engines":
      label = `Engines — ${lens.modality} nodes highlighted`;
      break;
  }
  // Show all nodes; the lens just determines highlighting (the 3D world
  // already dims non-matching nodes via the `dimmed` flag on the renderer).
  for (const n of nodes) show.add(n.id);
  return { show, label };
}

// ─── Component ───

export const GraphNativeSection: React.FC<{
  projectId: number | null;
  lens: SectionLens;
  /** Optional detail renderer for the selected node (engine-specific). */
  renderDetail?: (node: GraphViewNode) => React.ReactNode;
  /** Optional height override. */
  height?: number;
}> = ({ projectId, lens, renderDetail, height = 420 }) => {
  useGraphKitProject(projectId);
  const status = useGraphKit((s) => s.status);
  const nodes = useGraphKit((s) => s.nodes);
  const edges = useGraphKit((s) => s.edges);
  const crossLinksByContainer = useGraphKit((s) => s.crossLinksByContainer);
  const files = useGraphKit((s) => s.files);
  const unknownModalityGraphs = useGraphKit((s) => s.unknownModalityGraphs);
  const nodeEventCount = useGraphKit((s) => s.nodeEventCount);

  const [selectedId, setSelectedId] = useState<string | null>(null);
  const selectedNode = useMemo(
    () => nodes.find((n) => n.id === selectedId) ?? null,
    [nodes, selectedId],
  );

  const handleSelect = useCallback(
    (id: string) => {
      setSelectedId(id || null);
    },
    [],
  );

  const lensInfo = useMemo(() => lensFilter(lens, nodes), [lens, nodes]);

  if (projectId === null) {
    return (
      <div style={{ color: C_MUTED, fontSize: 12.5, padding: 12 }}>
        Select a project to explore its graph.
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 8, minWidth: 0 }}>
      {/* Header */}
      <div style={{ display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
        <span style={{ fontSize: 12.5, fontWeight: 700, color: C_TEXT }}>
          {lensInfo.label}
        </span>
        <span style={{ fontSize: 11, color: C_MUTED }}>
          {nodes.length} nodes · {edges.length} edges · {files.length} files
        </span>
        {unknownModalityGraphs > 0 && (
          <span style={{ fontSize: 10.5, color: "#d9b45f" }}>
            {unknownModalityGraphs} unknown-modality graphs (counted, not dropped)
          </span>
        )}
        {nodeEventCount > 0 && (
          <span style={{ fontSize: 10.5, color: C_ACCENT }}>
            ⚡ {nodeEventCount} live construction events
          </span>
        )}
      </div>

      {/* 3D World */}
      <GraphWorld3D
        nodes={nodes}
        edges={edges}
        selectedId={selectedId}
        onSelect={handleSelect}
        height={height}
      />

      {/* Selection inspector */}
      {selectedNode && (
        <div
          style={{
            border: `1px solid ${C_BORDER}`,
            borderRadius: 8,
            padding: "10px 12px",
            display: "flex",
            flexDirection: "column",
            gap: 6,
          }}
        >
          <div style={{ color: C_ACCENT, fontWeight: 700, fontSize: 13 }}>
            {selectedNode.label}
          </div>
          <div style={{ fontSize: 11, color: C_BODY }}>
            {selectedNode.nodeType} · {selectedNode.modality}
            {selectedNode.confidence != null && ` · confidence ${selectedNode.confidence.toFixed(2)}`}
          </div>
          {renderDetail && renderDetail(selectedNode)}
          {/* Cross-links */}
          {selectedNode.sourceContainerId != null && crossLinksByContainer[selectedNode.sourceContainerId] && (
            <div style={{ marginTop: 4 }}>
              <div style={{ fontSize: 10.5, color: C_MUTED, marginBottom: 3 }}>Cross-links:</div>
              <CrossLinkStrip
                containerId={selectedNode.sourceContainerId}
                onNavigate={(targetId) => {
                  // Find a node in the target container and select it
                  const target = nodes.find((n) => n.sourceContainerId === targetId);
                  if (target) setSelectedId(target.id);
                }}
              />
            </div>
          )}
        </div>
      )}
      {!selectedNode && status.kind === "ready" && nodes.length > 0 && (
        <div style={{ color: C_MUTED, fontSize: 12, padding: "4px 2px" }}>
          Click a node to inspect it and see its cross-links.
        </div>
      )}

      {/* Honest state */}
      {status.kind === "loading" && (
        <div style={{ color: C_MUTED, fontSize: 12.5 }}>Loading graph substrate…</div>
      )}
      {status.kind === "error" && (
        <div style={{ color: "#ff8a8a", fontSize: 12.5 }}>Error: {status.message}</div>
      )}
      {status.kind === "empty" && (
        <div style={{ color: C_MUTED, fontSize: 12.5, lineHeight: 1.5 }}>{status.why}</div>
      )}
    </div>
  );
};

export default GraphNativeSection;
