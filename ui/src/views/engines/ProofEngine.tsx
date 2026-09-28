/**
 * G3 — Proof/Formula Engine shell (Math).
 *
 * Composition fork: reuses the real math graph data/classification already
 * built for the Graph View (`loadGraphData`, `graphRenderers/mathNodes.ts` /
 * `mathEdges.ts`) and the real content renderer (`./MathViewer`) instead of
 * re-implementing any of it.
 *
 * Real node types (mathNodes.ts, construction-verified against
 * assets/pipelines/modalities/math/main.rs): Root, ProofStep, Variable,
 * Assumption. Real edge types (mathEdges.ts, build_math_graph): Contains,
 * FollowsStep, Uses, Defines, AssumesIn, DischargesIn.
 *
 * ORDERING, verified against every real on-disk graph
 * (assets/pipelines/modalities/math/zsei_data/graphs/math_*.json, 9 files
 * checked directly, not assumed): `ProofStep.step_number` is present and
 * chronological on every step, and matches node-id order and FollowsStep
 * direction whenever FollowsStep is present. FollowsStep itself is real
 * (main.rs:2479) but sparse in current data — only 2 of the 9 real graphs
 * contain any FollowsStep edge at all (the rest are 2-step proofs with no
 * step-to-step edge yet); direction is (later step -> earlier step), e.g.
 * real edges (3,2) and (4,3). Sequencing here uses `step_number` as the
 * primary, always-present order key (never array order), and separately
 * renders any real FollowsStep edges between consecutive steps as a
 * structural confirmation, not as the sequencing mechanism itself, since it
 * would leave most real steps unordered. `Uses` (Contains/Defines/AssumesIn/
 * DischargesIn) edges are shown per-step as real content citations, drawn
 * with the SAME `edgeVisual` styling the Graph View tab uses, so encoding
 * stays consistent across the app rather than inventing a second palette.
 */
import React, { useEffect, useState } from "react";
import { loadGraphData } from "../../graphViewData";
import { edgeVisual, nodeVisual } from "../../graphRenderers";
import { GraphViewEdge, GraphViewNode, GraphViewStatus } from "../../graphViewTypes";
import MathViewer from "../files/MathViewer";

const C_TEXT = "#dfe7f2";
const C_BODY = "#c7d0dc";
const C_MUTED = "#8b98ab";
const C_BORDER = "#1e2836";
const C_WARN = "#e8c14f";

interface ProofEngineProps {
  projectId: number | null;
}

function stepNumberOf(n: GraphViewNode): number | null {
  const v = n.raw.step_number;
  return typeof v === "number" ? v : null;
}

function edgeGlyph(edgeType: string): string {
  switch (edgeType) {
    case "Uses":
      return "cites";
    case "Defines":
      return "defines";
    case "AssumesIn":
      return "assumes";
    case "DischargesIn":
      return "discharges";
    default:
      return edgeType;
  }
}

export const ProofEngine: React.FC<ProofEngineProps> = ({ projectId }) => {
  const [status, setStatus] = useState<GraphViewStatus>({ kind: "loading" });
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);

  useEffect(() => {
    if (projectId === null) {
      setStatus({ kind: "empty" });
      return;
    }
    setSelectedNodeId(null);
    return loadGraphData(projectId, setStatus, { modality: "math" });
  }, [projectId]);

  if (projectId === null) {
    return <div style={{ padding: 16, color: C_MUTED, fontSize: 12.5 }}>Select a project to view its proof tree.</div>;
  }
  if (status.kind === "loading") {
    return <div style={{ padding: 16, color: C_MUTED, fontSize: 12.5 }}>Loading math graph…</div>;
  }
  if (status.kind === "error") {
    return <div style={{ padding: 16, color: "#ff8a8a", fontSize: 12.5 }}>Error: {status.message}</div>;
  }
  if (status.kind === "empty") {
    return (
      <div style={{ padding: 16, color: C_MUTED, fontSize: 12.5 }}>
        No math graph exists for this project yet — nothing fabricated to show in its place.
      </div>
    );
  }

  const nodes = status.data.nodes;
  const edges = status.data.edges;
  const nodeById = new Map(nodes.map((n) => [n.id, n] as const));

  const roots = nodes.filter((n) => n.nodeType === "Root");
  const steps = nodes
    .filter((n) => n.nodeType === "ProofStep")
    .slice()
    .sort((a, b) => {
      const sa = stepNumberOf(a);
      const sb = stepNumberOf(b);
      if (sa !== null && sb !== null) return sa - sb;
      if (sa !== null) return -1;
      if (sb !== null) return 1;
      return a.id.localeCompare(b.id);
    });
  const otherNodes = nodes.filter((n) => n.nodeType !== "Root" && n.nodeType !== "ProofStep");

  const followsStepByTarget = new Map<string, GraphViewEdge>();
  for (const e of edges) {
    if (e.edgeType === "FollowsStep") followsStepByTarget.set(e.from, e);
  }
  const edgesByFrom = new Map<string, GraphViewEdge[]>();
  for (const e of edges) {
    const list = edgesByFrom.get(e.from) ?? [];
    list.push(e);
    edgesByFrom.set(e.from, list);
  }

  const selected = selectedNodeId ? nodeById.get(selectedNodeId) ?? null : null;

  return (
    <div style={{ display: "flex", gap: 14 }}>
      <div style={{ flex: 1, minWidth: 0 }}>
        {roots.length === 0 && (
          <div style={{ fontSize: 11.5, color: C_MUTED, marginBottom: 8 }}>
            No `Root` node found in this graph — showing proof steps directly.
          </div>
        )}
        {roots.map((root) => (
          <div key={root.id} style={{ fontSize: 12.5, fontWeight: 700, color: C_TEXT, marginBottom: 8 }}>
            {root.label || "Proof"}
            {!root.isReal && (
              <span style={{ marginLeft: 8, fontSize: 10.5, color: C_WARN }}>schema-only type</span>
            )}
          </div>
        ))}

        {steps.length === 0 ? (
          <div style={{ fontSize: 12, color: C_MUTED }}>No proof steps recorded in this graph.</div>
        ) : (
          <ol style={{ listStyle: "none", margin: 0, padding: 0, display: "flex", flexDirection: "column", gap: 6 }}>
            {steps.map((step, i) => {
              const prev = i > 0 ? steps[i - 1] : null;
              const structuralLink = prev ? followsStepByTarget.get(step.id) : undefined;
              const structuralLinksToPrev = structuralLink?.to === prev?.id;
              const citations = (edgesByFrom.get(step.id) ?? []).filter((e) => e.edgeType !== "FollowsStep");
              const isSelected = step.id === selectedNodeId;
              return (
                <li key={step.id}>
                  {prev && (
                    <div style={{ fontSize: 10.5, color: C_MUTED, marginLeft: 26, marginBottom: 2 }}>
                      {structuralLinksToPrev ? (
                        <span title="Real FollowsStep edge from this step to the previous one">
                          ↑ FollowsStep (structural)
                        </span>
                      ) : (
                        <span title="No FollowsStep edge recorded between these two steps in this graph — ordered here by the real step_number field only">
                          ↑ ordered by step_number (no FollowsStep edge recorded)
                        </span>
                      )}
                    </div>
                  )}
                  <div
                    onClick={() => setSelectedNodeId(step.id === selectedNodeId ? null : step.id)}
                    style={{
                      display: "flex",
                      gap: 10,
                      alignItems: "flex-start",
                      padding: "6px 10px",
                      borderRadius: 8,
                      border: `1px solid ${isSelected ? C_TEXT : C_BORDER}`,
                      background: isSelected ? "rgba(255,255,255,0.04)" : "transparent",
                      cursor: "pointer",
                    }}
                  >
                    <span
                      style={{
                        flexShrink: 0,
                        width: 22,
                        height: 22,
                        borderRadius: "50%",
                        background: nodeVisual(step).fill,
                        color: "#0a0f1a",
                        fontSize: 11,
                        fontWeight: 700,
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "center",
                      }}
                    >
                      {stepNumberOf(step) ?? "?"}
                    </span>
                    <div style={{ minWidth: 0 }}>
                      <div style={{ fontSize: 12.5, color: C_TEXT }}>
                        {step.label || `Step ${stepNumberOf(step) ?? ""}`}
                        {!step.isReal && <span style={{ marginLeft: 8, fontSize: 10.5, color: C_WARN }}>schema-only</span>}
                      </div>
                      {step.contentPreview && (
                        <div style={{ fontSize: 11.5, color: C_BODY, marginTop: 2 }}>{step.contentPreview}</div>
                      )}
                      {citations.length > 0 && (
                        <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginTop: 4 }}>
                          {citations.map((c) => {
                            const target = nodeById.get(c.to);
                            const v = edgeVisual(c, "math");
                            return (
                              <span
                                key={c.id}
                                title={`${c.edgeType} → ${target?.label ?? c.to}`}
                                style={{
                                  fontSize: 10.5,
                                  padding: "1px 6px",
                                  borderRadius: 999,
                                  border: `1px solid ${v.stroke}`,
                                  color: v.stroke,
                                  opacity: c.isReal ? 1 : 0.5,
                                }}
                              >
                                {edgeGlyph(c.edgeType)}
                                {target ? `: ${target.label}` : ""}
                              </span>
                            );
                          })}
                        </div>
                      )}
                    </div>
                  </div>
                </li>
              );
            })}
          </ol>
        )}

        {otherNodes.length > 0 && (
          <div style={{ marginTop: 14 }}>
            <div style={{ fontSize: 11, fontWeight: 700, textTransform: "uppercase", color: C_MUTED, marginBottom: 6 }}>
              Other nodes ({otherNodes.length})
            </div>
            <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
              {otherNodes.map((n) => (
                <span
                  key={n.id}
                  onClick={() => setSelectedNodeId(n.id === selectedNodeId ? null : n.id)}
                  title={n.nodeType}
                  style={{
                    fontSize: 11,
                    padding: "2px 8px",
                    borderRadius: 999,
                    border: `1px solid ${C_BORDER}`,
                    color: n.isReal ? C_BODY : C_MUTED,
                    opacity: n.isReal ? 1 : 0.6,
                    cursor: "pointer",
                  }}
                >
                  {n.label}
                </span>
              ))}
            </div>
          </div>
        )}
      </div>

      <div style={{ width: 320, flexShrink: 0 }}>
        {selected && (
          <div style={{ marginBottom: 12, fontSize: 11.5, color: C_BODY }}>
            <div style={{ fontWeight: 700, color: C_TEXT, marginBottom: 4 }}>Selected node</div>
            <div>Type: {selected.nodeType}</div>
            <div>Label: {selected.label}</div>
            {selected.confidence !== undefined && <div>Confidence: {selected.confidence.toFixed(2)}</div>}
          </div>
        )}
        <MathViewer projectId={projectId} />
      </div>
    </div>
  );
};

export default ProofEngine;
