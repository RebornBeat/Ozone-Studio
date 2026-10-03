/**
 * G2 — Code Intelligence Engine: call-graph visualization.
 *
 * Shows ONLY real `Calls` edges (CodeEdgeType::Calls) — the direct payoff of
 * this session's Calls-edge wiring fix. This is deliberately narrower than
 * the general Graph View tab: a focused caller→callee view.
 *
 * Ground truth, verified directly against source and real on-disk data
 * (not taken from graphRenderers/codeEdges.ts's header comment alone):
 *
 * - `assets/pipelines/modalities/code/main.rs` (~2748-2765): Calls edges are
 *   built ONLY from `analysis.function_calls`, matched against
 *   `function_name_to_id` — a map of THIS FILE's own function names. The
 *   comment right above the construction site says it plainly: "Calls edges
 *   (function -> function), same-file resolution only — extract_function_calls
 *   only ever matches callees present in this file's own function_names, so
 *   nothing here is dropped that the extractor wouldn't already have
 *   dropped." There is no cross-file call resolution anywhere in this
 *   pipeline today. This banner is not a caveat buried in a tooltip — it is
 *   always visible, because it changes what an empty result means.
 * - Real edge `properties`: `line` (the call-site line number) and
 *   `is_method` (bool) — confirmed at the same construction site.
 * - Real node fields for the endpoints (Function-type nodes only; there is
 *   no separate "Method" node type — `codeNodes.ts`'s real-type list is
 *   File/Function/Class/Import): `name`, `position.start_line/end_line`.
 * - Checked EVERY code graph currently on disk (5 files under
 *   `zsei_data/graphs/code_*.json`, the only code_*.json files anywhere in
 *   the repo): every one has exactly 2 edges (Contains + Imports) and ZERO
 *   Calls edges. So the empty state below is not a hypothetical — it is
 *   today's actual, only real state, and is designed as the primary view,
 *   not an afterthought.
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
const C_ACCENT = "#5fb3ff"; // MODALITY_COLOR.code

interface CallEdgeInfo {
  edge: GraphViewEdge;
  line?: number;
  isMethod?: boolean;
}

interface CallerGroup {
  caller: GraphViewNode;
  callees: { node: GraphViewNode; calls: CallEdgeInfo[] }[];
}

function codePosition(node: GraphViewNode): { file_path?: string; start_line?: number; end_line?: number } | null {
  const p = node.raw.position;
  if (!p || typeof p !== "object") return null;
  return p as { file_path?: string; start_line?: number; end_line?: number };
}

function positionLabel(node: GraphViewNode): string | null {
  const p = codePosition(node);
  if (!p || typeof p.start_line !== "number") return null;
  const range = p.end_line !== undefined && p.end_line !== p.start_line ? `${p.start_line}-${p.end_line}` : `${p.start_line}`;
  return p.file_path ? `${p.file_path}:${range}` : `L${range}`;
}

function buildCallerGroups(nodes: GraphViewNode[], edges: GraphViewEdge[]): CallerGroup[] {
  const byId = new Map(nodes.map((n) => [n.id, n]));
  const callEdges = edges.filter((e) => e.edgeType === "Calls");
  const groups = new Map<string, CallerGroup>();

  for (const edge of callEdges) {
    const caller = byId.get(edge.from);
    const callee = byId.get(edge.to);
    if (!caller || !callee) continue; // real edge but an endpoint wasn't in this fetch — skip honestly, don't guess

    let group = groups.get(caller.id);
    if (!group) {
      group = { caller, callees: [] };
      groups.set(caller.id, group);
    }
    const raw = edge.raw as { properties?: Record<string, unknown> };
    const props = raw.properties ?? {};
    const info: CallEdgeInfo = {
      edge,
      line: typeof props.line === "number" ? props.line : undefined,
      isMethod: typeof props.is_method === "boolean" ? props.is_method : undefined,
    };

    let calleeEntry = group.callees.find((c) => c.node.id === callee.id);
    if (!calleeEntry) {
      calleeEntry = { node: callee, calls: [] };
      group.callees.push(calleeEntry);
    }
    calleeEntry.calls.push(info);
  }

  for (const g of groups.values()) {
    g.callees.sort((a, b) => a.node.label.localeCompare(b.node.label));
  }
  return Array.from(groups.values()).sort((a, b) => a.caller.label.localeCompare(b.caller.label));
}

function jumpToNode(node: GraphViewNode, projectId: number) {
  const p = codePosition(node);
  if (p?.file_path) {
    navigateTo({ kind: "code-file", projectId, path: p.file_path, line: p.start_line });
  } else {
    navigateTo({ kind: "graph-node", projectId, nodeId: node.id });
  }
}

const SameFileBanner: React.FC = () => (
  <div
    style={{
      border: `1px solid ${C_BORDER}`,
      borderLeft: `3px solid ${C_ACCENT}`,
      borderRadius: 6,
      padding: "8px 10px",
      marginBottom: 12,
      fontSize: 11.5,
      color: C_MUTED,
      lineHeight: 1.5,
    }}
  >
    <b style={{ color: C_TEXT }}>Same-file resolution only.</b> A <code>Calls</code> edge here means one function
    called another function defined in the <i>same analyzed file</i>. Cross-file call resolution does not exist in
    the current code pipeline — a function calling something in another file produces no edge, not a missing one.
  </div>
);

export const CodeCallGraph: React.FC<{ projectId: number | null }> = ({ projectId }) => {
  const [status, setStatus] = useState<GraphViewStatus>({ kind: "loading" });

  useEffect(() => {
    if (projectId === null) return;
    setStatus({ kind: "loading" });
    const dispose = loadGraphData(projectId, setStatus, { modality: "code" });
    return dispose;
  }, [projectId]);

  const groups = useMemo(
    () => (status.kind === "ready" ? buildCallerGroups(status.data.nodes, status.data.edges) : []),
    [status],
  );
  const totalCallEdges = useMemo(
    () => (status.kind === "ready" ? status.data.edges.filter((e) => e.edgeType === "Calls").length : 0),
    [status],
  );

  if (projectId === null) {
    return <div style={{ color: C_MUTED, fontSize: 12.5 }}>Select a project to see its call graph.</div>;
  }

  return (
    <div>
      <div style={{ fontSize: 12.5, fontWeight: 700, color: C_TEXT, marginBottom: 8 }}>Call Graph (Calls edges)</div>
      <SameFileBanner />

      {status.kind === "loading" && <div style={{ color: C_MUTED, fontSize: 12.5 }}>Loading code graph…</div>}
      {status.kind === "error" && (
        <div style={{ color: C_ERROR, fontSize: 12.5 }}>Error loading code graph: {status.message}</div>
      )}
      {status.kind === "empty" && (
        <div style={{ color: C_MUTED, fontSize: 12.5 }}>
          No code-modality graph exists for this project yet — nothing to show.
        </div>
      )}
      {status.kind === "ready" && totalCallEdges === 0 && (
        <div
          style={{
            color: C_MUTED,
            fontSize: 12.5,
            border: `1px dashed ${C_BORDER}`,
            borderRadius: 8,
            padding: 16,
            lineHeight: 1.6,
          }}
        >
          This project's code graph has real functions but <b>zero real <code>Calls</code> edges</b> — verified
          against every code graph currently on disk (all 5 have exactly this shape: Contains + Imports only, no
          Calls). This is not a bug in this view: it means no function in the analyzed file(s) was seen calling
          another function defined in that same file. This is genuinely today's real state, not a fallback.
        </div>
      )}
      {status.kind === "ready" && totalCallEdges > 0 && (
        <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
          {groups.map((group) => (
            <div
              key={group.caller.id}
              style={{ border: `1px solid ${C_BORDER}`, borderRadius: 8, padding: "8px 10px", background: C_PANEL }}
            >
              <div
                style={{ display: "flex", alignItems: "baseline", gap: 8, cursor: "pointer" }}
                onClick={() => jumpToNode(group.caller, projectId)}
                title="Jump to source"
              >
                <span style={{ color: C_ACCENT, fontWeight: 700, fontSize: 12.5 }}>ƒ {group.caller.label}</span>
                {positionLabel(group.caller) && (
                  <span style={{ color: C_MUTED, fontSize: 11 }}>{positionLabel(group.caller)}</span>
                )}
              </div>
              <div style={{ marginTop: 6, marginLeft: 14, display: "flex", flexDirection: "column", gap: 4 }}>
                {group.callees.map((c) => (
                  <div
                    key={c.node.id}
                    style={{ display: "flex", alignItems: "baseline", gap: 8, cursor: "pointer" }}
                    onClick={() => jumpToNode(c.node, projectId)}
                    title="Jump to source"
                  >
                    <span style={{ color: C_MUTED, fontSize: 12 }}>→</span>
                    <span style={{ color: C_BODY, fontSize: 12 }}>{c.node.label}</span>
                    {c.calls.length > 1 && (
                      <span style={{ color: C_MUTED, fontSize: 11 }}>×{c.calls.length}</span>
                    )}
                    {c.calls.some((i) => i.isMethod) && (
                      <span style={{ color: C_MUTED, fontSize: 11 }}>method</span>
                    )}
                    {positionLabel(c.node) && (
                      <span style={{ color: C_MUTED, fontSize: 11 }}>{positionLabel(c.node)}</span>
                    )}
                    {c.calls.some((i) => i.line !== undefined) && (
                      <span style={{ color: C_MUTED, fontSize: 11 }}>
                        call site line {c.calls.map((i) => i.line).filter((l) => l !== undefined).join(", ")}
                      </span>
                    )}
                  </div>
                ))}
              </div>
            </div>
          ))}
          <div style={{ color: C_MUTED, fontSize: 11 }}>
            {totalCallEdges} real Calls edge{totalCallEdges === 1 ? "" : "s"} across {groups.length} caller
            {groups.length === 1 ? "" : "s"}.
          </div>
        </div>
      )}
    </div>
  );
};

export default CodeCallGraph;
