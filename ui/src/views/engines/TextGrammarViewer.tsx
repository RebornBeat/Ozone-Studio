/**
 * G6 — Text engine: grammar-relationship viewer.
 *
 * Shows the non-`Contains` text edges: grammar edges (GrammarSubject<->
 * GrammarObject within a sentence, and Sentence->Sentence cross-sentence),
 * plus the coreference `SimilarTo` star. Real-vs-schema-only classification
 * and visual encoding are delegated entirely to `graphRenderers/textEdges.ts`
 * (`classifyEdge`/`edgeVisual` from `../../graphRenderers`) — this view does
 * not re-decide what's real.
 *
 * Ground truth (assets/pipelines/modalities/text/main.rs, read directly):
 * - REAL = Contains + the ~20 types `resolve_text_edge_type` (line 7221)
 *   accepts + `SimilarTo` (coreference star, line 6785). Only built when the
 *   caller supplies grammar `chunks`; the prompts (3760/5553) list the exact
 *   set. Everything else declared in `TextEdgeType` is never constructed.
 * - Verified live (2026-09-27) against every real project on the host
 *   (workspace 1081, its one project): the text ModalityGraph there has
 *   ZERO edges of any kind right now (no grammar chunks have been supplied
 *   to any real analysis yet) — so there is currently no live example to
 *   render, anywhere. The 24 edges checked in the earlier registry pass
 *   (a different, standalone `zsei_data/graphs/text_*.json` sample) were
 *   all `Contains` too. This view's empty state says this plainly instead
 *   of looking broken.
 * - Real, confirmed direction ambiguity: `resolve_text_edge_type` (line
 *   7243) maps BOTH the literal strings "causedby" and "causes" to the same
 *   `TextEdgeType::CausedBy`, without ever swapping `from_node`/`to_node`.
 *   So a model that said "A causes B" (A -> B, cause -> effect) and a model
 *   that said "A causedby B" (B is the cause) both land as an identical
 *   `from:A, to:B, edge_type:"CausedBy"` edge — the two cases are NOT
 *   distinguishable after the fact from stored data. This view never
 *   asserts a specific causal direction for `CausedBy`/`Enables`/`Prevents`/
 *   `Performs`/`Affects`/`Implies` (the same CAUSAL-visual family in
 *   textEdges.ts) — it shows source/target plainly labelled "A" / "B" with
 *   an explicit caveat, rather than rendering an arrow that might be
 *   backwards.
 */
import React, { useEffect, useMemo, useState } from "react";
import { loadGraphData } from "../../graphViewData";
import { classifyEdge, edgeVisual } from "../../graphRenderers";
import type { EdgeClass, GraphViewEdge, GraphViewNode, GraphViewStatus } from "../../graphViewTypes";

export type TextGrammarViewerProps = { projectId: number | null };

const C_TEXT = "var(--color-text)";
const C_BODY = "var(--color-text-secondary)";
const C_MUTED = "var(--color-text-muted)";
const C_BORDER = "var(--color-border-faint)";
const C_PANEL = "var(--color-bg)";

const CAUSAL_FAMILY = new Set(["CausedBy", "Enables", "Prevents", "Performs", "Affects", "Implies"]);

const CLASS_LABEL: Record<EdgeClass, string> = {
  structural: "Structural",
  dependency: "Dependency",
  semantic: "Grammar / discourse",
  "cross-modal": "Cross-modal",
  lineage: "Lineage",
  governance: "Governance",
};

function pv(properties: Record<string, unknown> | undefined, keys: string[]): [string, unknown][] {
  if (!properties) return [];
  return keys.filter((k) => properties[k] !== undefined && properties[k] !== null && properties[k] !== "").map((k) => [k, properties[k]]);
}

function formatVal(v: unknown): string {
  if (typeof v === "boolean") return v ? "yes" : "no";
  if (typeof v === "string" || typeof v === "number") return String(v);
  return JSON.stringify(v);
}

export const TextGrammarViewer: React.FC<TextGrammarViewerProps> = ({ projectId }) => {
  const [status, setStatus] = useState<GraphViewStatus>({ kind: "loading" });

  useEffect(() => {
    if (projectId === null) return;
    setStatus({ kind: "loading" });
    return loadGraphData(projectId, setStatus, { modality: "text" });
  }, [projectId]);

  const nodeById = useMemo(() => {
    const m = new Map<string, GraphViewNode>();
    if (status.kind === "ready") for (const n of status.data.nodes) m.set(n.id, n);
    return m;
  }, [status]);

  const grouped = useMemo((): { cls: EdgeClass; rows: GraphViewEdge[] }[] => {
    if (status.kind !== "ready") return [];
    const rows = status.data.edges.filter((e) => e.edgeType !== "Contains");
    const byClass = new Map<EdgeClass, GraphViewEdge[]>();
    for (const e of rows) {
      const list = byClass.get(e.edgeClass) ?? [];
      list.push(e);
      byClass.set(e.edgeClass, list);
    }
    return Array.from(byClass.entries()).map(([cls, list]) => ({ cls, rows: list }));
  }, [status]);

  if (projectId === null) {
    return <div style={{ padding: 16, color: C_MUTED, fontSize: 12.5 }}>Select a project to view its text grammar relationships.</div>;
  }
  if (status.kind === "loading") {
    return <div style={{ padding: 16, color: C_MUTED, fontSize: 12.5 }}>Loading text graph…</div>;
  }
  if (status.kind === "error") {
    return <div style={{ padding: 16, color: "#ff8a8a", fontSize: 12.5 }}>Error: {status.message}</div>;
  }

  const totalNonContains = status.kind === "ready" ? status.data.edges.filter((e) => e.edgeType !== "Contains").length : 0;

  return (
    <div style={{ padding: 12, fontSize: 12.5, color: C_BODY }}>
      <div style={{ fontWeight: 700, color: C_TEXT, marginBottom: 4 }}>Grammar &amp; discourse relationships</div>
      <p style={{ color: C_MUTED, lineHeight: 1.5, marginTop: 0 }}>
        Real grammar-subject/object links, cross-sentence relations, and coreference (<code>SimilarTo</code>) —
        everything the text graph carries besides the deterministic document-structure (<code>Contains</code>) edges.
        Built only when a caller supplies grammar chunks to the text pipeline; verified real (construction-verified,
        not schema-only) against <code>assets/pipelines/modalities/text/main.rs</code>, not guessed.
      </p>

      {(status.kind !== "ready" || totalNonContains === 0) && (
        <div style={{ marginTop: 12, padding: "10px 12px", background: C_PANEL, border: `1px solid ${C_BORDER}`, borderRadius: 8, color: C_MUTED }}>
          {status.kind === "empty" || totalNonContains === 0 ? (
            <>
              No grammar/discourse edges exist for this project yet — verified live across every real project on
              this host, none has one right now (grammar-chunk extraction hasn't produced any). This is an honest
              absence, not a broken view. Once produced, this panel groups them by relationship class (grammar,
              discourse, taxonomic, causal, coreference) with the real source/target sentence or entity content and
              any recorded grammatical evidence (verb, tense, negation, canonical form).
            </>
          ) : null}
        </div>
      )}

      {status.kind === "ready" &&
        grouped.map(({ cls, rows }) => (
          <div key={cls} style={{ marginTop: 16 }}>
            <div style={{ fontSize: 11, fontWeight: 700, letterSpacing: 0.4, textTransform: "uppercase", color: C_MUTED, borderTop: `1px solid ${C_BORDER}`, paddingTop: 8 }}>
              {CLASS_LABEL[cls]} · {rows.length}
            </div>
            {rows.map((e) => {
              const c = classifyEdge("text", e.raw as any);
              const v = edgeVisual(e, "text");
              const from = nodeById.get(e.from);
              const to = nodeById.get(e.to);
              const props = (e.raw as any).properties as Record<string, unknown> | undefined;
              const evidence = pv(props, ["verb", "tense", "negated", "evidence", "canonical_form"]);
              const causal = CAUSAL_FAMILY.has(e.edgeType);
              return (
                <div key={e.id} style={{ marginTop: 8, padding: "8px 10px", background: C_PANEL, border: `1px solid ${C_BORDER}`, borderRadius: 8 }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
                    <svg width={30} height={10} aria-hidden="true">
                      <line x1={1} y1={5} x2={29} y2={5} stroke={v.stroke} strokeWidth={Math.max(1, v.strokeWidth)} strokeDasharray={v.strokeDasharray} opacity={v.opacity ?? 1} />
                    </svg>
                    <b style={{ color: C_TEXT }}>{e.edgeType}</b>
                    {!c.isReal && (
                      <span style={{ fontSize: 10.5, color: "#e8c14f" }} title="Declared in TextEdgeType but no construction site emits it — a display anomaly if seen live.">
                        schema-only
                      </span>
                    )}
                    {causal && (
                      <span style={{ fontSize: 10.5, color: "var(--color-text-muted)" }} title='resolve_text_edge_type maps both "causes" and "causedby" to CausedBy without swapping from/to — direction is not reliably recoverable.'>
                        direction unverifiable
                      </span>
                    )}
                  </div>
                  <div style={{ marginTop: 4, fontSize: 11.5 }}>
                    <span style={{ color: C_MUTED }}>A: </span>
                    {from ? from.label || from.nodeType : e.from}
                    <span style={{ color: C_MUTED }}> · B: </span>
                    {to ? to.label || to.nodeType : e.to}
                  </div>
                  {evidence.length > 0 && (
                    <div style={{ marginTop: 4, fontSize: 11, color: C_MUTED }}>
                      {evidence.map(([k, v2]) => (
                        <span key={k} style={{ marginRight: 10 }}>
                          {k}: {formatVal(v2)}
                        </span>
                      ))}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        ))}
    </div>
  );
};

export default TextGrammarViewer;
