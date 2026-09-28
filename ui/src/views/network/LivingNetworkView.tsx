/**
 * J1 — Living Network View shell.
 *
 * A higher-level, "is anything moving right now" overview of one project —
 * AMT generations + per-modality graph clusters + live update pulses — NOT a
 * duplicate of the full Graph View tab (that already shows every real node
 * and edge; this shows shape and activity at a glance).
 *
 * Real data sources, all already landed:
 *  - `loadGraphData` (../../graphViewData, B1/B0) — per-modality node/edge
 *    counts. It already re-fetches itself on a live graph_event scoped to
 *    this project, so this view gets that part of "living" for free.
 *  - `loadAmtGenerations` (../../data/amtLineage, D4) — real AMT generations
 *    (main/fork), with an honest `lineageSource` per generation.
 *  - `getGraphEventClient` (../../graphEventClient, B19) — a SEPARATE
 *    subscription here (loadGraphData's internal one already triggers its
 *    own re-fetch; this one exists purely to drive the visible pulse
 *    animation on a real frame, not to fetch again).
 *
 * Blueprint linkage (the "Blueprint" quarter of AMT↔Graph↔Steps↔Blueprint):
 * investigated, NOT built. Per docs/UI_UX_FORK_PLAN.md's B17 finding,
 * blueprint containers are code-complete but their real content files
 * (`zsei_data/blueprints/*.json`) do not exist on disk — only `index.json`
 * does, so `GetContainerContent` on any real blueprint container returns a
 * StorageError today. There is also no real edge/keyword in this codebase
 * linking a specific project to a specific blueprint container (blueprints
 * are a separate top-level tree, not project-scoped). Showing a "Blueprint"
 * node here would have to be fabricated, so this view states the gap
 * honestly instead (see the dedicated note in the render below) rather than
 * drawing a node with nothing real behind it.
 *
 * "Steps" (the fourth quarter): a project's real step activity lives on
 * individual TASKS (`/task/get`), not on the project container itself, and
 * no real query ties "all tasks for project X" together (confirmed: `Task`
 * records carry no `project_id` field — the same gap H4 already found from
 * the other direction). So this view shows a real, honest placeholder for
 * that quarter too, rather than inventing a join that doesn't exist.
 */
import React, { useEffect, useMemo, useRef, useState } from "react";
import { loadGraphData } from "../../graphViewData";
import { GraphViewStatus, Modality } from "../../graphViewTypes";
import { MODALITY_COLOR } from "../../graphRenderers/defaults";
import { loadAmtGenerations, AmtGeneration } from "../../data/amtLineage";
import { getGraphEventClient, GraphEventFrame } from "../../graphEventClient";

const C_TEXT = "#dfe7f2";
const C_BODY = "#c7d0dc";
const C_MUTED = "#8b98ab";
const C_BORDER = "#1e2836";
const C_PANEL = "#0a0f1a";
const C_WARN = "#e8c14f";

interface Props {
  projectId: number | null;
}

interface ModalityCount {
  modality: Modality;
  nodes: number;
  edges: number;
}

function summarizeByModality(status: GraphViewStatus): ModalityCount[] {
  if (status.kind !== "ready") return [];
  const byModality = new Map<Modality, { nodes: number; edges: number }>();
  for (const n of status.data.nodes) {
    const s = byModality.get(n.modality as Modality) ?? { nodes: 0, edges: 0 };
    s.nodes += 1;
    byModality.set(n.modality as Modality, s);
  }
  for (const e of status.data.edges) {
    // edge ids are built as `${modality}:${containerId}:...` by graphViewData.ts
    const modality = e.id.split(":")[0] as Modality;
    const s = byModality.get(modality) ?? { nodes: 0, edges: 0 };
    s.edges += 1;
    byModality.set(modality, s);
  }
  return Array.from(byModality.entries()).map(([modality, c]) => ({ modality, ...c }));
}

/** A ring of small circles, purely a "how much is here" glance — not a claim
 * about real layout geometry (same disclaimer as GraphView.tsx's canvas). */
const RingCluster: React.FC<{
  cx: number;
  cy: number;
  label: string;
  color: string;
  count: number;
  pulsing: boolean;
}> = ({ cx, cy, label, color, count, pulsing }) => {
  const r = count === 0 ? 0 : Math.min(34, 10 + Math.sqrt(count) * 4);
  return (
    <g transform={`translate(${cx} ${cy})`}>
      {pulsing && (
        <circle r={r + 6} fill="none" stroke={color} strokeWidth={2} opacity={0.55}>
          <animate attributeName="r" from={r} to={r + 22} dur="0.9s" begin="0s" repeatCount="1" />
          <animate attributeName="opacity" from="0.55" to="0" dur="0.9s" begin="0s" repeatCount="1" />
        </circle>
      )}
      <circle r={r} fill={color} opacity={count === 0 ? 0.12 : 0.85} stroke={color} strokeWidth={1} />
      <text y={r + 16} textAnchor="middle" fontSize={11} fill={C_BODY}>
        {label}
      </text>
      <text y={4} textAnchor="middle" fontSize={11} fill="#0a0f1a" fontWeight={700}>
        {count > 0 ? count : ""}
      </text>
    </g>
  );
};

const AmtBadge: React.FC<{ gen: AmtGeneration; pulsing: boolean }> = ({ gen, pulsing }) => (
  <div
    style={{
      display: "flex",
      alignItems: "center",
      gap: 6,
      padding: "3px 8px",
      borderRadius: 999,
      border: `1px solid ${C_BORDER}`,
      background: pulsing ? "rgba(232,193,79,0.15)" : "transparent",
      fontSize: 11.5,
      color: C_BODY,
      transition: "background 0.4s ease",
    }}
    title={`container ${gen.containerId} — lineage: ${gen.lineageSource}`}
  >
    <span style={{ fontWeight: 700, color: gen.isMain ? "#8fe38f" : "#ffb95f" }}>
      {gen.isMain ? "main" : "fork"}
    </span>
    <span>#{gen.containerId}</span>
    {gen.lineageSource === "keyword" && (
      <span style={{ color: C_MUTED, fontStyle: "italic" }} title="legacy keyword-only lineage, no recorded confidence">
        (legacy link)
      </span>
    )}
    {gen.isDuplicateMain && (
      <span style={{ color: C_WARN }} title="more than one amt-main generation under this project">
        dup-main
      </span>
    )}
  </div>
);

export const LivingNetworkView: React.FC<Props> = ({ projectId }) => {
  const [graphStatus, setGraphStatus] = useState<GraphViewStatus>({ kind: "loading" });
  const [generations, setGenerations] = useState<AmtGeneration[] | null>(null);
  const [genError, setGenError] = useState<string | null>(null);
  const [pulseAt, setPulseAt] = useState<number>(0);

  useEffect(() => {
    if (projectId === null) return;
    return loadGraphData(projectId, setGraphStatus);
  }, [projectId]);

  useEffect(() => {
    if (projectId === null) return;
    let cancelled = false;
    setGenerations(null);
    setGenError(null);
    loadAmtGenerations(projectId)
      .then((g) => {
        if (!cancelled) setGenerations(g);
      })
      .catch((e) => {
        if (!cancelled) setGenError(e instanceof Error ? e.message : String(e));
      });
    return () => {
      cancelled = true;
    };
  }, [projectId]);

  // Separate subscription purely for the visual pulse — loadGraphData already
  // re-fetches itself on the same real event; this just makes that moment
  // visible instead of silent.
  useEffect(() => {
    if (projectId === null) return;
    const client = getGraphEventClient();
    client.connect();
    return client.onEvent(
      (frame: GraphEventFrame) => {
        if (frame.parent_id === projectId || frame.container_id === projectId) {
          setPulseAt(Date.now());
        }
      },
      [`proj:${projectId}`, "scope:global"],
    );
  }, [projectId]);

  // Pulse is "on" for a short real window after a real event, not looped.
  const [pulsing, setPulsing] = useState(false);
  useEffect(() => {
    if (pulseAt === 0) return;
    setPulsing(true);
    const t = setTimeout(() => setPulsing(false), 1000);
    return () => clearTimeout(t);
  }, [pulseAt]);

  const modalityCounts = useMemo(() => summarizeByModality(graphStatus), [graphStatus]);
  const layout = useMemo(() => {
    const modalities: Modality[] = ["code", "math", "text"];
    return modalities.map((m, i) => {
      const found = modalityCounts.find((c) => c.modality === m);
      const angle = (2 * Math.PI * i) / modalities.length - Math.PI / 2;
      return {
        modality: m,
        nodes: found?.nodes ?? 0,
        edges: found?.edges ?? 0,
        cx: 140 + 90 * Math.cos(angle),
        cy: 110 + 90 * Math.sin(angle),
      };
    });
  }, [modalityCounts]);

  if (projectId === null) {
    return <div style={{ padding: 16, color: C_MUTED, fontSize: 12.5 }}>Select a project to see its living network.</div>;
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
      <div>
        <div style={{ fontSize: 12.5, fontWeight: 700, color: C_TEXT, marginBottom: 6 }}>Graph activity</div>
        {graphStatus.kind === "loading" && <div style={{ fontSize: 12, color: C_MUTED }}>Loading…</div>}
        {graphStatus.kind === "error" && <div style={{ fontSize: 12, color: "#ff8a8a" }}>Error: {graphStatus.message}</div>}
        {graphStatus.kind === "empty" && (
          <div style={{ fontSize: 12, color: C_MUTED }}>No modality graphs exist for this project yet.</div>
        )}
        {(graphStatus.kind === "ready" || graphStatus.kind === "empty") && (
          <svg width={280} height={230} style={{ background: C_PANEL, border: `1px solid ${C_BORDER}`, borderRadius: 10 }}>
            {layout.map((l) => (
              <RingCluster
                key={l.modality}
                cx={l.cx}
                cy={l.cy}
                label={`${l.modality} (${l.edges} edges)`}
                color={MODALITY_COLOR[l.modality]}
                count={l.nodes}
                pulsing={pulsing}
              />
            ))}
          </svg>
        )}
        <div style={{ fontSize: 11, color: C_MUTED, marginTop: 4 }}>
          Live via the real graph_event socket — a ring pulses on any real update scoped to this project.
        </div>
      </div>

      <div>
        <div style={{ fontSize: 12.5, fontWeight: 700, color: C_TEXT, marginBottom: 6 }}>AMT generations</div>
        {generations === null && !genError && <div style={{ fontSize: 12, color: C_MUTED }}>Loading…</div>}
        {genError && <div style={{ fontSize: 12, color: "#ff8a8a" }}>Error: {genError}</div>}
        {generations && generations.length === 0 && (
          <div style={{ fontSize: 12, color: C_MUTED }}>No AMT generations recorded for this project yet.</div>
        )}
        {generations && generations.length > 0 && (
          <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
            {generations.map((g) => (
              <AmtBadge key={g.containerId} gen={g} pulsing={pulsing} />
            ))}
          </div>
        )}
      </div>

      <div style={{ fontSize: 11.5, color: C_MUTED, borderTop: `1px solid ${C_BORDER}`, paddingTop: 8, lineHeight: 1.5 }}>
        <b style={{ color: C_WARN }}>Blueprint &amp; Steps:</b> not shown here. Blueprint content files don't exist on
        disk for any real blueprint container yet (a confirmed data gap, not a code gap), and there is no real link
        from a project to a specific blueprint container in this codebase. Task/step activity has no real
        project-scoped query either — a `Task` record carries no `project_id`. Showing either would mean fabricating
        a link that doesn't exist, so both are left out rather than faked.
      </div>
    </div>
  );
};

export default LivingNetworkView;
