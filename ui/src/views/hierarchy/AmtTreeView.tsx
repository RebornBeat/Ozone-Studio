/**
 * D3 — Hierarchy View: real AMT tree.
 *
 * Real sources:
 *  - Generations: data/amtLineage.ts `loadAmtGenerations` (B6/B7) — every AMT generation is a `Derived`
 *    container that is a direct child of its project, or of container 3 when the request had no project.
 *  - Tree: data/amtTree.ts `loadAmtTree` → `GetContainerContent` (B0). `children` nest full `AMTNode`s
 *    (src/orchestrator/mod.rs:614; confirmed against target/release/zsei_data/amt/*.json — 27 of 61 files
 *    have branches, 244 nodes total, types seen: Root/Branch/Leaf/Consideration, max depth 3).
 *  - Graph side: graphViewData.ts `loadGraphData` (B1) for the project's persisted modality graphs.
 *
 * Alignment with graph nodes — what is REAL and what is not:
 *  - AMT nodes reference source material ONLY by `source_chunk_indices` (u32[]); AMT↔AMT links are
 *    `relationships[]` (only `Continues` → prior generation container occurs on disk).
 *  - Text-graph nodes have a `source_chunk_index` field in their schema, but no persisted text-graph node
 *    on disk carries a value for it (0 of 38 checked), and nothing else in AMT references graph node ids.
 *    So NO AMT↔graph link is recorded in current data. This view therefore shows the graph summary beside the
 *    tree and states that plainly. A chunk-index match is displayed ONLY if graph nodes actually carry one,
 *    and is labelled as an index coincidence, not a recorded link.
 *  - `confidence` on an AMT node is a derived 1.0/0.0 mirror of `verified` (struct doc comment), so it is
 *    shown as verified/unverified, never as an independent score.
 */
import React, { useEffect, useMemo, useState } from "react";
import {
  AmtGeneration,
  UNATTACHED_AMT_PARENT_ID,
  canonicalMain,
  loadAmtGenerations,
} from "../../data/amtLineage";
import { AmtNodeRaw, FlatAmtNode, flattenAmtTree, loadAmtTree } from "../../data/amtTree";
import { loadGraphData } from "../../graphViewData";
import type { GraphViewNode, GraphViewStatus } from "../../graphViewTypes";

export type AmtTreeViewProps = { projectId: number | null };

const C_TEXT = "var(--color-text)";
const C_BODY = "var(--color-text-secondary)";
const C_MUTED = "var(--color-text-muted)";
const C_BORDER = "var(--color-border-faint)";
const C_WARN = "#e8c14f";
const C_ERR = "#ff8a8a";

const NODE_TYPE_COLOR: Record<string, string> = {
  Root: "#e8c14f",
  Branch: "#5fb3ff",
  Leaf: "#8fe38f",
  Consideration: "#ffb95f",
  CrossReference: "#c792ea",
};
const wrap: React.CSSProperties = { overflowWrap: "anywhere", wordBreak: "break-word", minWidth: 0 };

type Scope = "project" | "unattached";

type Loadable<T> =
  | { kind: "idle" }
  | { kind: "loading" }
  | { kind: "error"; message: string }
  | { kind: "ready"; value: T };

function generationLabel(g: AmtGeneration): string {
  const role = g.isMain ? "main" : g.forkOf !== null ? `fork of #${g.forkOf}` : "generation";
  return `#${g.containerId} · ${role}${g.isDuplicateMain ? " (duplicate main)" : ""}`;
}

function preview(text: string, n = 90): string {
  const t = text.replace(/\s+/g, " ").trim();
  return t.length > n ? `${t.slice(0, n)}…` : t;
}

const Badge: React.FC<{ children: React.ReactNode; color?: string; title?: string; dashed?: boolean }> = ({
  children,
  color = C_MUTED,
  title,
  dashed,
}) => (
  <span
    title={title}
    style={{
      fontSize: 10.5,
      color,
      border: `1px ${dashed ? "dashed" : "solid"} ${color}`,
      borderRadius: 999,
      padding: "0 6px",
      whiteSpace: "nowrap",
    }}
  >
    {children}
  </span>
);

const Row: React.FC<{ label: string; children: React.ReactNode }> = ({ label, children }) => (
  <div style={{ ...wrap, marginBottom: 2 }}>
    <span style={{ color: C_MUTED }}>{label}: </span>
    <span style={{ color: C_BODY }}>{children}</span>
  </div>
);

const SectionTitle: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <div
    style={{
      fontSize: 11,
      fontWeight: 700,
      letterSpacing: 0.4,
      textTransform: "uppercase",
      color: C_MUTED,
      margin: "12px 0 4px",
      paddingTop: 8,
      borderTop: `1px solid ${C_BORDER}`,
    }}
  >
    {children}
  </div>
);

export const AmtTreeView: React.FC<AmtTreeViewProps> = ({ projectId }) => {
  const [scope, setScope] = useState<Scope>(projectId === null ? "unattached" : "project");
  useEffect(() => {
    // A project becoming available/unavailable should re-pick a sensible default scope.
    setScope(projectId === null ? "unattached" : "project");
  }, [projectId]);
  const parentId = scope === "project" ? projectId : UNATTACHED_AMT_PARENT_ID;

  const [gens, setGens] = useState<Loadable<AmtGeneration[]>>({ kind: "idle" });
  const [selectedGen, setSelectedGen] = useState<number | null>(null);
  const [tree, setTree] = useState<Loadable<AmtNodeRaw | null>>({ kind: "idle" });
  const [graph, setGraph] = useState<GraphViewStatus>({ kind: "loading" });
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [showRaw, setShowRaw] = useState(false);

  // Generations for the chosen parent container.
  useEffect(() => {
    if (parentId === null) {
      setGens({ kind: "idle" });
      setSelectedGen(null);
      return;
    }
    let cancelled = false;
    setGens({ kind: "loading" });
    loadAmtGenerations(parentId)
      .then((list) => {
        if (cancelled) return;
        // Main first, then the rest by container id.
        const sorted = [...list].sort((a, b) => Number(b.isMain) - Number(a.isMain) || a.containerId - b.containerId);
        setGens({ kind: "ready", value: sorted });
        const canon = canonicalMain(sorted);
        setSelectedGen((prev) =>
          prev !== null && sorted.some((g) => g.containerId === prev)
            ? prev
            : canon?.containerId ?? sorted[0]?.containerId ?? null,
        );
      })
      .catch((e) => !cancelled && setGens({ kind: "error", message: e instanceof Error ? e.message : String(e) }));
    return () => {
      cancelled = true;
    };
  }, [parentId]);

  // Tree of the selected generation.
  useEffect(() => {
    setSelectedPath(null);
    setShowRaw(false);
    if (selectedGen === null) {
      setTree({ kind: "idle" });
      return;
    }
    let cancelled = false;
    setTree({ kind: "loading" });
    loadAmtTree(selectedGen)
      .then((root) => {
        if (cancelled) return;
        setTree({ kind: "ready", value: root });
        // Root + its direct children visible by default; deeper levels open on demand.
        setExpanded(new Set(root ? ["0"] : []));
      })
      .catch((e) => !cancelled && setTree({ kind: "error", message: e instanceof Error ? e.message : String(e) }));
    return () => {
      cancelled = true;
    };
  }, [selectedGen]);

  // Real modality graphs of the selected project (only meaningful for a real project scope).
  useEffect(() => {
    if (scope !== "project" || projectId === null) return;
    setGraph({ kind: "loading" });
    return loadGraphData(projectId, setGraph);
  }, [scope, projectId]);

  const flat: FlatAmtNode[] = useMemo(
    () => (tree.kind === "ready" && tree.value ? flattenAmtTree(tree.value) : []),
    [tree],
  );
  const stats = useMemo(() => {
    if (flat.length === 0) return null;
    return {
      nodes: flat.length,
      maxDepth: Math.max(...flat.map((f) => f.level)),
      verified: flat.filter((f) => f.node.verified).length,
      byType: flat.reduce<Record<string, number>>((acc, f) => {
        acc[f.node.node_type] = (acc[f.node.node_type] ?? 0) + 1;
        return acc;
      }, {}),
    };
  }, [flat]);

  const selectedFlat = flat.find((f) => f.path === selectedPath) ?? null;
  const generations = gens.kind === "ready" ? gens.value : [];

  const toggle = (path: string) =>
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });

  // A path is visible when every ancestor path is expanded.
  const visible = flat.filter((f) => {
    const parts = f.path.split("/");
    for (let i = 1; i < parts.length; i++) {
      if (!expanded.has(parts.slice(0, i).join("/"))) return false;
    }
    return true;
  });

  // Graph-side facts for the alignment statement.
  const graphNodes: GraphViewNode[] = graph.kind === "ready" ? graph.data.nodes : [];
  const graphNodesWithChunkRef = graphNodes.filter((n) => typeof n.raw.source_chunk_index === "number");
  const chunkMatches =
    selectedFlat && selectedFlat.node.source_chunk_indices.length > 0
      ? graphNodesWithChunkRef.filter((n) => selectedFlat.node.source_chunk_indices.includes(n.raw.source_chunk_index as number))
      : [];

  return (
    <div style={{ color: C_BODY, fontSize: 12.5 }}>
      {/* Scope + generation selector */}
      <div style={{ display: "flex", gap: 12, alignItems: "center", flexWrap: "wrap", marginBottom: 10 }}>
        <label style={{ color: C_MUTED }}>Scope:</label>
        <select
          value={scope}
          onChange={(e) => setScope(e.target.value as Scope)}
          style={{ background: "#101724", color: C_TEXT, border: `1px solid ${C_BORDER}`, borderRadius: 6, padding: "4px 8px" }}
        >
          <option value="project" disabled={projectId === null}>
            Selected project{projectId !== null ? ` (#${projectId})` : " (none selected)"}
          </option>
          <option value="unattached">No-project generations (container {UNATTACHED_AMT_PARENT_ID})</option>
        </select>
        {generations.length > 0 && (
          <>
            <label style={{ color: C_MUTED }}>Generation:</label>
            <select
              value={selectedGen ?? undefined}
              onChange={(e) => setSelectedGen(Number(e.target.value))}
              style={{ background: "#101724", color: C_TEXT, border: `1px solid ${C_BORDER}`, borderRadius: 6, padding: "4px 8px", maxWidth: 320 }}
            >
              {generations.map((g) => (
                <option key={g.containerId} value={g.containerId}>
                  {generationLabel(g)}
                </option>
              ))}
            </select>
            <span style={{ color: C_MUTED }}>{generations.length} generation(s)</span>
          </>
        )}
      </div>

      {parentId === null && (
        <div style={{ color: C_MUTED }}>No project is selected — choose one above, or switch scope to the no-project generations.</div>
      )}
      {gens.kind === "loading" && <div style={{ color: C_MUTED }}>Loading AMT generations…</div>}
      {gens.kind === "error" && <div style={{ color: C_ERR }}>Error loading AMT generations: {gens.message}</div>}
      {gens.kind === "ready" && generations.length === 0 && (
        <div style={{ color: C_MUTED }}>
          No AMT generations are recorded under container #{parentId}. Nothing is fabricated in their place.
        </div>
      )}

      {selectedGen !== null && (
        <div style={{ display: "flex", gap: 12, alignItems: "flex-start", flexWrap: "wrap" }}>
          {/* Tree */}
          <div style={{ flex: "1 1 380px", minWidth: 280, border: `1px solid ${C_BORDER}`, borderRadius: 10, padding: 10 }}>
            <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 8, flexWrap: "wrap" }}>
              <span style={{ fontWeight: 700, color: C_TEXT }}>AMT tree · container #{selectedGen}</span>
              {stats && (
                <span style={{ color: C_MUTED, fontSize: 11.5 }}>
                  {stats.nodes} node(s) · depth {stats.maxDepth} · {stats.verified} verified ·{" "}
                  {Object.entries(stats.byType)
                    .map(([t, n]) => `${n} ${t}`)
                    .join(", ")}
                </span>
              )}
              {flat.length > 1 && (
                <span style={{ marginLeft: "auto", display: "flex", gap: 6 }}>
                  <button onClick={() => setExpanded(new Set(flat.filter((f) => f.node.children.length > 0).map((f) => f.path)))} style={miniBtn}>
                    Expand all
                  </button>
                  <button onClick={() => setExpanded(new Set(["0"]))} style={miniBtn}>
                    Collapse
                  </button>
                </span>
              )}
            </div>

            {tree.kind === "loading" && <div style={{ color: C_MUTED }}>Loading tree…</div>}
            {tree.kind === "error" && <div style={{ color: C_ERR }}>Error: {tree.message}</div>}
            {tree.kind === "ready" && tree.value === null && (
              <div style={{ color: C_MUTED }}>
                This container has no readable tree content (no content file, or it is not JSON).
              </div>
            )}
            {tree.kind === "ready" && tree.value && flat.length === 1 && (
              <div style={{ color: C_WARN, marginBottom: 8, fontSize: 12 }}>
                Root node only — no branches were recorded for this generation.
              </div>
            )}

            {visible.map((f) => {
              const n = f.node;
              const color = NODE_TYPE_COLOR[n.node_type];
              const isSel = f.path === selectedPath;
              const hasKids = n.children.length > 0;
              return (
                <div
                  key={f.path}
                  onClick={() => setSelectedPath(f.path)}
                  style={{
                    display: "flex",
                    gap: 6,
                    alignItems: "baseline",
                    padding: "3px 6px",
                    marginLeft: f.level * 16,
                    borderRadius: 6,
                    cursor: "pointer",
                    background: isSel ? "var(--color-border-faint)" : "transparent",
                  }}
                >
                  <span
                    onClick={(e) => {
                      e.stopPropagation();
                      if (hasKids) toggle(f.path);
                    }}
                    style={{ width: 12, color: C_MUTED, flexShrink: 0, cursor: hasKids ? "pointer" : "default" }}
                  >
                    {hasKids ? (expanded.has(f.path) ? "▾" : "▸") : ""}
                  </span>
                  <Badge color={color ?? C_MUTED} dashed={!color} title={color ? undefined : "Node type not in the declared AMTNodeType set"}>
                    {n.node_type}
                  </Badge>
                  <span style={{ ...wrap, color: C_TEXT, flex: 1 }}>{preview(n.content) || <i style={{ color: C_MUTED }}>(empty content)</i>}</span>
                  {n.verified ? (
                    <Badge color="#8fe38f" title="Backed by source chunk evidence">✓ verified</Badge>
                  ) : (
                    <Badge title="No source chunk evidence recorded for this node" dashed>unverified</Badge>
                  )}
                  {n.source_chunk_indices.length > 0 && <Badge title="Source chunk indices">{n.source_chunk_indices.length} chunk(s)</Badge>}
                  {n.methodology_ids.length > 0 && <Badge title="Linked methodology ids">{n.methodology_ids.length} meth</Badge>}
                  {n.relationships.length > 0 && <Badge title="Recorded relationships">{n.relationships.length} rel</Badge>}
                  {hasKids && <Badge title="Child nodes">{n.children.length} ↓</Badge>}
                </div>
              );
            })}
          </div>

          {/* Detail + graph alignment */}
          <div style={{ flex: "0 1 340px", minWidth: 260, border: `1px solid ${C_BORDER}`, borderRadius: 10, padding: 10 }}>
            <div style={{ fontWeight: 700, color: C_TEXT, marginBottom: 6 }}>Node detail</div>
            {!selectedFlat && <div style={{ color: C_MUTED }}>Select a node to inspect it.</div>}
            {selectedFlat && (
              <NodeDetail
                flat={selectedFlat}
                generations={generations}
                allFlat={flat}
                onSelectPath={setSelectedPath}
                onSelectGeneration={setSelectedGen}
                showRaw={showRaw}
                setShowRaw={setShowRaw}
              />
            )}

            <SectionTitle>Graph alignment</SectionTitle>
            <div style={{ ...wrap, fontSize: 12, lineHeight: 1.55 }}>
              {scope !== "project" && (
                <div style={{ color: C_MUTED }}>
                  No-project generations have no owning project, so there are no persisted modality graphs to relate them to.
                </div>
              )}
              {scope === "project" && graph.kind === "loading" && <div style={{ color: C_MUTED }}>Loading the project's graphs…</div>}
              {scope === "project" && graph.kind === "error" && <div style={{ color: C_ERR }}>Graph load error: {graph.message}</div>}
              {scope === "project" && graph.kind === "empty" && (
                <div style={{ color: C_MUTED }}>This project has no persisted modality graphs yet.</div>
              )}
              {scope === "project" && graph.kind === "ready" && (
                <>
                  <div style={{ color: C_MUTED }}>
                    Project graphs: {graph.data.nodes.length} node(s), {graph.data.edges.length} edge(s) across{" "}
                    {graph.data.sourceContainers.length} graph container(s).
                  </div>
                  {graphNodesWithChunkRef.length === 0 ? (
                    <div style={{ color: C_WARN, marginTop: 4 }}>
                      No AMT↔graph link is recorded: 0 of {graphNodes.length} graph node(s) carry a <code>source_chunk_index</code>, and
                      AMT nodes reference source material only by chunk index. Nothing has been aligned or invented.
                    </div>
                  ) : selectedFlat ? (
                    chunkMatches.length > 0 ? (
                      <div style={{ marginTop: 4 }}>
                        <div style={{ color: C_WARN }}>
                          {chunkMatches.length} graph node(s) share a source chunk index with this AMT node. This is an index
                          coincidence — chunk numbering across the AMT and modality pipelines is not verified to be the same.
                        </div>
                        {chunkMatches.slice(0, 20).map((n) => (
                          <div key={n.id} style={{ color: C_BODY }}>
                            · {n.modality} {n.nodeType}: {preview(n.label, 60)} (chunk {String(n.raw.source_chunk_index)})
                          </div>
                        ))}
                      </div>
                    ) : (
                      <div style={{ color: C_MUTED, marginTop: 4 }}>
                        {graphNodesWithChunkRef.length} graph node(s) carry a chunk index, none matching this node's chunks.
                      </div>
                    )
                  ) : null}
                </>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

const miniBtn: React.CSSProperties = {
  background: "transparent",
  color: C_MUTED,
  border: `1px solid ${C_BORDER}`,
  borderRadius: 6,
  padding: "1px 8px",
  fontSize: 11,
  cursor: "pointer",
};

const NodeDetail: React.FC<{
  flat: FlatAmtNode;
  generations: AmtGeneration[];
  allFlat: FlatAmtNode[];
  onSelectPath: (p: string) => void;
  onSelectGeneration: (id: number) => void;
  showRaw: boolean;
  setShowRaw: (b: boolean) => void;
}> = ({ flat, generations, allFlat, onSelectPath, onSelectGeneration, showRaw, setShowRaw }) => {
  const n = flat.node;
  const sentences = Object.entries(n.metadata).filter(([k]) => k.startsWith("source_sentence_"));
  const otherMeta = Object.entries(n.metadata).filter(([k]) => !k.startsWith("source_sentence_"));
  return (
    <div style={{ lineHeight: 1.55 }}>
      <Row label="Node id">{n.id}</Row>
      <Row label="Type">{n.node_type}</Row>
      <Row label="Depth">{n.depth}</Row>
      <Row label="Verified">
        {n.verified ? "yes — backed by source chunk evidence" : "no — no source chunk evidence recorded"}
      </Row>
      <div style={{ ...wrap, fontSize: 11, color: C_MUTED, marginBottom: 4 }}>
        (Stored confidence {n.confidence} is a derived mirror of “verified”, not an independent score.)
      </div>
      <SectionTitle>Content</SectionTitle>
      <div style={{ ...wrap, whiteSpace: "pre-wrap", maxHeight: 180, overflowY: "auto" }}>{n.content || <i style={{ color: C_MUTED }}>(empty)</i>}</div>

      {n.source_chunk_indices.length > 0 && (
        <>
          <SectionTitle>Source chunks</SectionTitle>
          <div>{n.source_chunk_indices.join(", ")}</div>
        </>
      )}
      {n.methodology_ids.length > 0 && (
        <>
          <SectionTitle>Methodology ids</SectionTitle>
          <div>{n.methodology_ids.join(", ")}</div>
        </>
      )}

      <SectionTitle>Relationships ({n.relationships.length})</SectionTitle>
      {n.relationships.length === 0 && <div style={{ color: C_MUTED }}>None recorded on this node.</div>}
      {n.relationships.map((r, i) => {
        const isContinues = r.relation_type === "Continues";
        const targetGen = isContinues ? generations.find((g) => g.containerId === r.target_id) : undefined;
        const targetNode = !isContinues ? allFlat.find((f) => f.node.id === r.target_id) : undefined;
        return (
          <div key={i} style={{ ...wrap, marginBottom: 3 }}>
            <b>{r.relation_type}</b> → {isContinues ? `prior AMT container #${r.target_id}` : `node #${r.target_id}`}{" "}
            <span style={{ color: C_MUTED }}>(confidence {r.confidence})</span>
            {targetGen && (
              <button onClick={() => onSelectGeneration(targetGen.containerId)} style={{ ...miniBtn, marginLeft: 6 }}>
                open
              </button>
            )}
            {isContinues && !targetGen && <span style={{ color: C_MUTED }}> — not among the listed generations</span>}
            {targetNode && (
              <button onClick={() => onSelectPath(targetNode.path)} style={{ ...miniBtn, marginLeft: 6 }}>
                go to node
              </button>
            )}
            {!isContinues && !targetNode && <span style={{ color: C_MUTED }}> — target not found in this tree</span>}
          </div>
        );
      })}

      {(sentences.length > 0 || otherMeta.length > 0) && <SectionTitle>Metadata</SectionTitle>}
      {sentences.map(([k, v]) => (
        <div key={k} style={{ ...wrap, marginBottom: 3 }}>
          <span style={{ color: C_MUTED }}>{k}: </span>
          <i>“{v}”</i>
        </div>
      ))}
      {otherMeta.map(([k, v]) => (
        <Row key={k} label={k}>
          {v}
        </Row>
      ))}

      <div style={{ marginTop: 10 }}>
        <button onClick={() => setShowRaw(!showRaw)} style={miniBtn}>
          {showRaw ? "Hide raw JSON" : "Show raw JSON"}
        </button>
      </div>
      {showRaw && (
        <pre
          style={{
            ...wrap,
            whiteSpace: "pre-wrap",
            background: "var(--color-bg)",
            border: `1px solid ${C_BORDER}`,
            borderRadius: 6,
            padding: 8,
            fontSize: 11,
            maxHeight: 240,
            overflow: "auto",
          }}
        >
          {JSON.stringify({ ...n, children: `[${n.children.length} child node(s)]` }, null, 2)}
        </pre>
      )}
    </div>
  );
};

export default AmtTreeView;
