/**
 * G4 — Proof/Formula Engine: variable scope tree.
 *
 * Real data only, verified directly against all 9 persisted math graphs on
 * disk (assets/pipelines/modalities/math/zsei_data/graphs/math_*.json):
 *   - node_type counts: ProofStep 19, Root 9, Variable 8. Zero `Assumption`
 *     nodes anywhere — `graphRenderers/mathNodes.ts` documents Assumption as
 *     real-but-gated (analyze_proof still hardcodes its source data empty),
 *     which matches: the constructor exists, but nothing has ever called it
 *     with data that produces one.
 *   - Every real `Defines` edge is ProofStep -> Variable (8/8 on disk) — this
 *     is the one real "who introduces this variable" link
 *     (graphRenderers/mathEdges.ts confirms Defines is real, pushed by
 *     build_math_graph). Used here as the sole real scoping mechanism.
 *   - The wrapper's `scope_tree` field is present in the schema but EMPTY on
 *     all 9 real graphs (0/9 non-empty) — NOT used for anything real here;
 *     stated honestly in the UI rather than silently ignored.
 *   - `Expression`/`Axiom`/`Theorem`/`Definition`/`Constant`/`Scope` have NO
 *     construction site anywhere in assets/pipelines/modalities/math/main.rs
 *     (confirmed again here, not just trusted from mathNodes.ts's comment) —
 *     shown as a single explicit "not implemented" note, not an empty
 *     section that could be mistaken for a bug or missing data.
 */
import React, { useEffect, useState } from "react";
import { loadGraphData } from "../../graphViewData";
import { GraphViewEdge, GraphViewNode, GraphViewStatus } from "../../graphViewTypes";

const UNCONSTRUCTED_TYPES = ["Expression", "Axiom", "Theorem", "Definition", "Constant", "Scope"] as const;

interface ScopeGroup {
  definer: GraphViewNode | null; // null => "no defining step found" bucket
  variables: GraphViewNode[];
}

function buildScopeGroups(nodes: GraphViewNode[], edges: GraphViewEdge[]): ScopeGroup[] {
  const variables = nodes.filter((n) => n.modality === "math" && n.nodeType === "Variable");
  const byId = new Map(nodes.map((n) => [n.id, n]));
  const definerOf = new Map<string, GraphViewNode>();
  for (const e of edges) {
    if (e.edgeType !== "Defines") continue;
    const from = byId.get(e.from);
    const to = byId.get(e.to);
    if (from?.nodeType === "ProofStep" && to?.nodeType === "Variable") definerOf.set(to.id, from);
  }
  const groups = new Map<string, ScopeGroup>();
  const unlinked: GraphViewNode[] = [];
  for (const v of variables) {
    const definer = definerOf.get(v.id) ?? null;
    if (!definer) {
      unlinked.push(v);
      continue;
    }
    const key = definer.id;
    if (!groups.has(key)) groups.set(key, { definer, variables: [] });
    groups.get(key)!.variables.push(v);
  }
  const ordered = Array.from(groups.values()).sort((a, b) => {
    const sa = typeof a.definer?.raw.step_number === "number" ? a.definer.raw.step_number : 0;
    const sb = typeof b.definer?.raw.step_number === "number" ? b.definer.raw.step_number : 0;
    return sa - sb;
  });
  if (unlinked.length > 0) ordered.push({ definer: null, variables: unlinked });
  return ordered;
}

function assumptionCount(nodes: GraphViewNode[]): number {
  return nodes.filter((n) => n.modality === "math" && n.nodeType === "Assumption").length;
}

const SectionNote: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <div style={{ fontSize: 12, color: "#8b98ab", lineHeight: 1.5, marginBottom: 12 }}>{children}</div>
);

export const VariableScopeTree: React.FC<{ projectId: number | null }> = ({ projectId }) => {
  const [status, setStatus] = useState<GraphViewStatus>({ kind: "loading" });

  useEffect(() => {
    if (projectId === null) return;
    return loadGraphData(projectId, setStatus, { modality: "math" });
  }, [projectId]);

  if (projectId === null) {
    return <div style={{ padding: 16, color: "#8b98ab", fontSize: 12.5 }}>Select a project to see its variable scope tree.</div>;
  }
  if (status.kind === "loading") {
    return <div style={{ padding: 16, color: "#8b98ab", fontSize: 12.5 }}>Loading math graph…</div>;
  }
  if (status.kind === "error") {
    return <div style={{ padding: 16, color: "#ff8a8a", fontSize: 12.5 }}>Error: {status.message}</div>;
  }
  if (status.kind === "empty") {
    return <div style={{ padding: 16, color: "#8b98ab", fontSize: 12.5 }}>No math graph exists for this project yet.</div>;
  }

  const { nodes, edges } = status.data;
  const groups = buildScopeGroups(nodes, edges);
  const assumptions = assumptionCount(nodes);

  return (
    <div style={{ padding: 4, fontSize: 12.5, color: "#c7d0dc" }}>
      <div style={{ fontWeight: 700, color: "#dfe7f2", marginBottom: 6 }}>Variable scope tree</div>
      <SectionNote>
        Scope shown here is the real <code>Defines</code> edge from a <code>ProofStep</code> to the{" "}
        <code>Variable</code> it introduces — the only construction-verified link between a variable and where it
        comes from. The graph's own <code>scope_tree</code> field is empty on every real graph checked, so it is not
        used here.
      </SectionNote>

      {groups.length === 0 && (
        <div style={{ color: "#8b98ab", fontStyle: "italic" }}>No real Variable nodes exist in this project's math graph yet.</div>
      )}

      {groups.map((g, i) => (
        <div key={g.definer?.id ?? `unlinked-${i}`} style={{ marginBottom: 10, borderLeft: "2px solid #1e2836", paddingLeft: 10 }}>
          <div style={{ color: g.definer ? "#dfe7f2" : "#ff8a8a", fontWeight: 600, marginBottom: 3 }}>
            {g.definer
              ? `Step ${typeof g.definer.raw.step_number === "number" ? g.definer.raw.step_number : "?"} — ${g.definer.label}`
              : "No defining step found"}
          </div>
          {g.variables.map((v) => (
            <div key={v.id} style={{ display: "flex", gap: 8, marginBottom: 2 }}>
              <span style={{ color: "#ffd699", fontFamily: "monospace" }}>{v.label}</span>
              {v.contentPreview && <span style={{ color: "#8b98ab" }}>{v.contentPreview}</span>}
            </div>
          ))}
        </div>
      ))}

      <div style={{ marginTop: 16, paddingTop: 10, borderTop: "1px solid #1e2836" }}>
        <div style={{ fontWeight: 700, color: "#dfe7f2", marginBottom: 4 }}>Assumptions</div>
        {assumptions > 0 ? (
          <div>{assumptions} real assumption node(s) in this project.</div>
        ) : (
          <SectionNote>
            No real <code>Assumption</code> nodes exist in this project yet. The type has a real constructor, but the
            analysis step that would supply assumption data (<code>analyze_proof</code>) still hardcodes it empty on
            the backend — this is a genuine current limitation, not a display bug.
          </SectionNote>
        )}
      </div>

      <div style={{ marginTop: 16, paddingTop: 10, borderTop: "1px solid #1e2836" }}>
        <div style={{ fontWeight: 700, color: "#dfe7f2", marginBottom: 4 }}>Axioms, theorems, definitions, constants, scopes</div>
        <SectionNote>
          Not shown: {UNCONSTRUCTED_TYPES.join(", ")} are declared node types in the math pipeline's schema, but
          nothing in the backend ever constructs one — there is no code path that produces this data today. This
          section is intentionally empty because the feature doesn't exist yet, not because of a bug.
        </SectionNote>
      </div>
    </div>
  );
};

export default VariableScopeTree;
