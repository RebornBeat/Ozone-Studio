/**
 * D4 — Hierarchy View: AMT fork/main lineage across generations.
 *
 * Real data only, from `data/amtLineage.ts` (B6/B7/B8: every AMT generation is a `Derived` container that is a
 * direct child of its project — or of container 3 when it was created with no project — carrying keyword
 * `amt-main` or `amt-fork-of:<id>`, plus real `ForkOf`/`ContinuedBy` relations on post-fix containers).
 *
 * Two lineage mechanisms coexist and are deliberately shown differently:
 *   - solid green connector / "edge" badge  → a real `ForkOf` relation is recorded (verified)
 *   - dashed amber connector / "keyword" badge → only the legacy one-directional `amt-fork-of:<id>` keyword exists
 * Nothing here is inferred beyond what the container records say; a fork whose parent isn't among the scanned
 * generations is shown detached, and a lineage cycle (never expected) is listed rather than hidden.
 */
import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  AmtGeneration,
  UNATTACHED_AMT_PARENT_ID,
  canonicalMain,
  childrenByParent,
  loadAmtGenerations,
  walkLineage,
} from "../../data/amtLineage";
import { getGraphEventClient } from "../../graphEventClient";

export interface ForkLineageViewProps {
  projectId: number | null;
}

const C = {
  text: "#dfe7f2",
  body: "#c7d0dc",
  muted: "#8b98ab",
  border: "#1e2836",
  panel: "#0a0f1a",
  edge: "#4fbf8f",
  keyword: "#e8c14f",
  bad: "#ff8a8a",
};

type Mode = "project" | "unattached";

const sourceColor = (s: AmtGeneration["lineageSource"]) =>
  s === "edge" || s === "both" ? C.edge : s === "keyword" ? C.keyword : C.muted;

const Badge: React.FC<{ color: string; dashed?: boolean; title?: string; children: React.ReactNode }> = ({
  color,
  dashed,
  title,
  children,
}) => (
  <span
    title={title}
    style={{
      fontSize: 10.5,
      color,
      border: `1px ${dashed ? "dashed" : "solid"} ${color}`,
      borderRadius: 999,
      padding: "1px 7px",
      whiteSpace: "nowrap",
    }}
  >
    {children}
  </span>
);

function sourceBadge(g: AmtGeneration): React.ReactNode {
  switch (g.lineageSource) {
    case "edge":
      return (
        <Badge color={C.edge} title="A real ForkOf relation is recorded on this container (verified lineage).">
          ForkOf edge
        </Badge>
      );
    case "both":
      return (
        <Badge color={C.edge} title="Both a real ForkOf relation and the amt-fork-of keyword are recorded.">
          edge + keyword
        </Badge>
      );
    case "keyword":
      return (
        <Badge
          color={C.keyword}
          dashed
          title="Only the legacy one-directional amt-fork-of:<id> keyword records this link — this container predates the ForkOf relation mirroring."
        >
          keyword only (legacy)
        </Badge>
      );
    default:
      return (
        <Badge color={C.muted} title="No parent link is recorded on this container.">
          no parent link
        </Badge>
      );
  }
}

const GenCard: React.FC<{ g: AmtGeneration; selected: boolean; onSelect: () => void }> = ({ g, selected, onSelect }) => (
  <div
    onClick={onSelect}
    style={{
      cursor: "pointer",
      background: selected ? "#141c2b" : C.panel,
      border: `1px solid ${selected ? "#5fb3ff" : C.border}`,
      borderRadius: 8,
      padding: "6px 10px",
      display: "flex",
      flexWrap: "wrap",
      gap: 6,
      alignItems: "center",
      minWidth: 0,
    }}
  >
    <b style={{ color: C.text, fontSize: 12.5 }}>#{g.containerId}</b>
    {g.isMain ? (
      <Badge color="#5fb3ff" title="Carries the amt-main keyword — the first generation of its project.">
        MAIN
      </Badge>
    ) : (
      <Badge color={C.body} title="A fork: an island spawned from a prior generation.">
        FORK
      </Badge>
    )}
    {g.isDuplicateMain && (
      <Badge color={C.bad} title="Another amt-main with a lower container id exists under the same parent (duplicate-main finding).">
        duplicate main
      </Badge>
    )}
    {!g.isMain && sourceBadge(g)}
    {g.lineageConflict && (
      <Badge color={C.bad} title={`ForkOf edge names #${g.forkOfEdgeId} but the keyword names #${g.forkOfKeywordId}.`}>
        edge/keyword conflict
      </Badge>
    )}
    {g.name && <span style={{ color: C.muted, fontSize: 11.5, overflowWrap: "anywhere" }}>{g.name}</span>}
    {g.continuedBy.length > 0 && (
      <span style={{ color: C.muted, fontSize: 11 }}>
        → {g.continuedBy.length} fork{g.continuedBy.length === 1 ? "" : "s"}
      </span>
    )}
  </div>
);

const GenTree: React.FC<{
  g: AmtGeneration;
  kids: Map<number, AmtGeneration[]>;
  selectedId: number | null;
  onSelect: (id: number) => void;
  path: Set<number>;
}> = ({ g, kids, selectedId, onSelect, path }) => {
  const children = (kids.get(g.containerId) ?? []).filter((c) => !path.has(c.containerId));
  const nextPath = new Set(path).add(g.containerId);
  return (
    <div>
      <GenCard g={g} selected={selectedId === g.containerId} onSelect={() => onSelect(g.containerId)} />
      {children.length > 0 && (
        <div style={{ marginLeft: 14, paddingLeft: 12, marginTop: 4, display: "flex", flexDirection: "column", gap: 4 }}>
          {children.map((c) => (
            <div
              key={c.containerId}
              style={{
                borderLeft: `2px ${c.lineageSource === "keyword" ? "dashed" : "solid"} ${sourceColor(c.lineageSource)}`,
                paddingLeft: 10,
              }}
            >
              <GenTree g={c} kids={kids} selectedId={selectedId} onSelect={onSelect} path={nextPath} />
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

const Row: React.FC<{ label: string; children: React.ReactNode }> = ({ label, children }) => (
  <div style={{ overflowWrap: "anywhere", marginBottom: 3 }}>
    <span style={{ color: C.muted }}>{label}: </span>
    <span style={{ color: C.body }}>{children}</span>
  </div>
);

const DetailPanel: React.FC<{ g: AmtGeneration | null; all: AmtGeneration[] }> = ({ g, all }) => {
  if (!g) return <div style={{ color: C.muted, fontSize: 12 }}>Select a generation to inspect its lineage records.</div>;
  const chain = walkLineage(all, g.containerId);
  const ids = (xs: number[] | undefined) => (xs && xs.length ? xs.map((x) => `#${x}`).join(", ") : "none");
  const inferredOnly = g.continuedBy.filter((c) => !(g.continuedByEdgeIds ?? []).includes(c));
  return (
    <div style={{ fontSize: 12, lineHeight: 1.55 }}>
      <Row label="Container">#{g.containerId}</Row>
      <Row label="Role">{g.isMain ? "main (amt-main)" : "fork"}</Row>
      <Row label="Parent container">#{g.projectId}</Row>
      <Row label="Forked from">{g.forkOf === null ? "— (no parent link)" : `#${g.forkOf}${g.danglingParent ? " (not among scanned generations)" : ""}`}</Row>
      <Row label="Lineage source">{g.lineageSource}</Row>
      <Row label="ForkOf edge → ">{g.forkOfEdgeId != null ? `#${g.forkOfEdgeId}` : "not recorded"}</Row>
      <Row label="amt-fork-of keyword → ">{g.forkOfKeywordId != null ? `#${g.forkOfKeywordId}` : "not recorded"}</Row>
      <Row label="ContinuedBy edges recorded">{ids(g.continuedByEdgeIds)}</Row>
      <Row label="Forks of this generation">{ids(g.continuedBy)}</Row>
      {inferredOnly.length > 0 && (
        <div style={{ color: C.muted, fontSize: 11, marginBottom: 3 }}>
          {ids(inferredOnly)} come from those forks' own parent links — no reverse ContinuedBy edge is recorded here.
        </div>
      )}
      {g.lineageConflict && (
        <div style={{ color: C.bad, marginTop: 4 }}>
          Edge and keyword disagree (#{g.forkOfEdgeId} vs #{g.forkOfKeywordId}); the edge is shown as authoritative.
        </div>
      )}
      {g.isDuplicateMain && (
        <div style={{ color: C.bad, marginTop: 4 }}>A lower-id amt-main exists under the same parent; this one is a duplicate.</div>
      )}
      <div style={{ marginTop: 10, color: C.muted, fontWeight: 700, fontSize: 11, textTransform: "uppercase", letterSpacing: 0.4 }}>
        Path to main (nearest first)
      </div>
      <div style={{ color: C.body, overflowWrap: "anywhere" }}>
        {chain.map((c) => `#${c.containerId}`).join(" ← ")}
        {chain.length > 0 && !chain[chain.length - 1].isMain && (
          <span style={{ color: C.keyword }}> (chain ends before reaching a main generation)</span>
        )}
      </div>
      <div style={{ marginTop: 10, color: C.muted, fontWeight: 700, fontSize: 11, textTransform: "uppercase", letterSpacing: 0.4 }}>
        Keywords
      </div>
      <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginTop: 3 }}>
        {g.keywords.map((k) => (
          <span key={k} style={{ fontSize: 11, color: C.body, border: `1px solid ${C.border}`, borderRadius: 4, padding: "0 5px" }}>
            {k}
          </span>
        ))}
      </div>
    </div>
  );
};

type LoadState =
  | { kind: "loading" }
  | { kind: "error"; message: string }
  | { kind: "ready"; generations: AmtGeneration[] };

export const ForkLineageView: React.FC<ForkLineageViewProps> = ({ projectId }) => {
  const [mode, setMode] = useState<Mode>(projectId !== null ? "project" : "unattached");
  useEffect(() => {
    setMode(projectId !== null ? "project" : "unattached");
  }, [projectId]);

  const scanId = mode === "project" && projectId !== null ? projectId : UNATTACHED_AMT_PARENT_ID;
  const [state, setState] = useState<LoadState>({ kind: "loading" });
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const knownIds = useRef<Set<number>>(new Set());

  const load = useCallback(
    (showSpinner: boolean) => {
      let cancelled = false;
      if (showSpinner) setState({ kind: "loading" });
      loadAmtGenerations(scanId)
        .then((generations) => {
          if (cancelled) return;
          knownIds.current = new Set(generations.map((g) => g.containerId));
          setState({ kind: "ready", generations });
        })
        .catch((e) => !cancelled && setState({ kind: "error", message: e instanceof Error ? e.message : String(e) }));
      return () => {
        cancelled = true;
      };
    },
    [scanId],
  );

  useEffect(() => {
    setSelectedId(null);
    return load(true);
  }, [load]);

  // Live refresh: a graph_event that touches the scanned container or a known generation re-fetches (debounced).
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | null = null;
    const client = getGraphEventClient();
    client.connect();
    const off = client.onEvent((frame) => {
      const touches =
        frame.parent_id === scanId ||
        frame.container_id === scanId ||
        knownIds.current.has(frame.container_id) ||
        knownIds.current.has(frame.parent_id);
      if (!touches) return;
      if (timer) clearTimeout(timer);
      timer = setTimeout(() => load(false), 600);
    });
    return () => {
      if (timer) clearTimeout(timer);
      off();
    };
  }, [scanId, load]);

  const generations = state.kind === "ready" ? state.generations : [];
  const kids = useMemo(() => childrenByParent(generations), [generations]);
  const { roots, unreachable } = useMemo(() => {
    const canon = canonicalMain(generations);
    const rootList = generations
      .filter((g) => g.forkOf === null || g.danglingParent)
      .sort((a, b) => {
        if (canon && a.containerId === canon.containerId) return -1;
        if (canon && b.containerId === canon.containerId) return 1;
        return a.containerId - b.containerId;
      });
    const seen = new Set<number>();
    const visit = (g: AmtGeneration) => {
      if (seen.has(g.containerId)) return;
      seen.add(g.containerId);
      (kids.get(g.containerId) ?? []).forEach(visit);
    };
    rootList.forEach(visit);
    return { roots: rootList, unreachable: generations.filter((g) => !seen.has(g.containerId)) };
  }, [generations, kids]);

  const stats = useMemo(() => {
    const forks = generations.filter((g) => !g.isMain);
    return {
      total: generations.length,
      mains: generations.filter((g) => g.isMain).length,
      edge: forks.filter((g) => g.lineageSource === "edge" || g.lineageSource === "both").length,
      keywordOnly: forks.filter((g) => g.lineageSource === "keyword").length,
      conflicts: generations.filter((g) => g.lineageConflict).length,
    };
  }, [generations]);

  const selected = generations.find((g) => g.containerId === selectedId) ?? null;
  const scanLabel = scanId === UNATTACHED_AMT_PARENT_ID ? "container 3 (generations created without a project)" : `project ${scanId}`;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 10, minHeight: 0 }}>
      <div style={{ display: "flex", gap: 10, alignItems: "center", flexWrap: "wrap", fontSize: 12 }}>
        <span style={{ color: C.muted }}>Scanning {scanLabel}</span>
        {projectId !== null && (
          <button
            onClick={() => setMode(mode === "project" ? "unattached" : "project")}
            style={{ background: "transparent", color: C.body, border: `1px solid ${C.border}`, borderRadius: 6, padding: "2px 9px", fontSize: 12, cursor: "pointer" }}
          >
            {mode === "project" ? "Show generations with no project" : "Show selected project"}
          </button>
        )}
        <button
          onClick={() => load(true)}
          style={{ background: "transparent", color: C.body, border: `1px solid ${C.border}`, borderRadius: 6, padding: "2px 9px", fontSize: 12, cursor: "pointer" }}
        >
          Refresh
        </button>
      </div>

      {state.kind === "loading" && <div style={{ color: C.muted, fontSize: 12.5 }}>Loading AMT generations…</div>}
      {state.kind === "error" && <div style={{ color: C.bad, fontSize: 12.5 }}>Error: {state.message}</div>}

      {state.kind === "ready" && generations.length === 0 && (
        <div style={{ color: C.muted, fontSize: 12.5 }}>
          No AMT generations exist under {scanLabel}. Nothing is drawn in their place.
          {mode === "project" && (
            <>
              {" "}
              <button
                onClick={() => setMode("unattached")}
                style={{ background: "transparent", color: "#5fb3ff", border: "none", cursor: "pointer", fontSize: 12.5, padding: 0 }}
              >
                Check generations created without a project
              </button>
            </>
          )}
        </div>
      )}

      {state.kind === "ready" && generations.length > 0 && (
        <>
          <div style={{ display: "flex", gap: 14, flexWrap: "wrap", fontSize: 11.5, color: C.muted }}>
            <span>{stats.total} generations</span>
            <span>{stats.mains} main</span>
            <span style={{ color: C.edge }}>{stats.edge} fork{stats.edge === 1 ? "" : "s"} with ForkOf edge</span>
            <span style={{ color: C.keyword }}>{stats.keywordOnly} keyword-only (legacy)</span>
            {stats.conflicts > 0 && <span style={{ color: C.bad }}>{stats.conflicts} conflict{stats.conflicts === 1 ? "" : "s"}</span>}
            <span>
              connector: <span style={{ color: C.edge }}>━ solid = ForkOf edge</span>{" "}
              <span style={{ color: C.keyword }}>╍ dashed = keyword only</span>
            </span>
          </div>
          <div style={{ display: "flex", gap: 12, minHeight: 0, alignItems: "flex-start" }}>
            <div style={{ flex: 1, minWidth: 0, display: "flex", flexDirection: "column", gap: 8 }}>
              {roots.map((r) => (
                <div key={r.containerId}>
                  {r.danglingParent && (
                    <div style={{ color: C.keyword, fontSize: 11, marginBottom: 2 }}>
                      Detached: parent #{r.forkOf} is not among the scanned generations
                    </div>
                  )}
                  <GenTree g={r} kids={kids} selectedId={selectedId} onSelect={setSelectedId} path={new Set()} />
                </div>
              ))}
              {unreachable.length > 0 && (
                <div>
                  <div style={{ color: C.bad, fontSize: 11, marginBottom: 2 }}>
                    Not reachable from any root (lineage cycle in the records):
                  </div>
                  {unreachable.map((g) => (
                    <div key={g.containerId} style={{ marginBottom: 4 }}>
                      <GenCard g={g} selected={selectedId === g.containerId} onSelect={() => setSelectedId(g.containerId)} />
                    </div>
                  ))}
                </div>
              )}
            </div>
            <div style={{ width: 300, flexShrink: 0, border: `1px solid ${C.border}`, borderRadius: 10, padding: 12 }}>
              <div style={{ fontSize: 12.5, fontWeight: 700, color: C.text, marginBottom: 8 }}>Generation detail</div>
              <DetailPanel g={selected} all={generations} />
            </div>
          </div>
        </>
      )}
    </div>
  );
};

export default ForkLineageView;
