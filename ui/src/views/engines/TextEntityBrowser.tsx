/**
 * G5 — Text engine: Entity/Topic/Keyword browser.
 *
 * Data: the project's real persisted text graph via `loadGraphData(projectId,
 * cb, {modality:"text"})` (../../graphViewData) — the same B1 fetch sequence
 * and C6/C7 real-vs-schema-only classification the Graph View canvas uses.
 *
 * Real construction sites confirmed in `graphRenderers/textNodes.ts`'s own
 * header comment (assets/pipelines/modalities/text/main.rs `create_graph`):
 * Document/Section/Entity/Topic/Keyword are always built; Sentence/
 * GrammarSubject/GrammarObject only when grammar chunks are supplied. This
 * browser only lists Entity/Topic/Keyword, per the fork directive.
 *
 * Real per-node fields (verified directly against on-disk graphs, e.g.
 * assets/pipelines/modalities/text/zsei_data/graphs/text_1789777161917145828.json):
 *   Entity  — content = the entity name; properties.entity_type,
 *             properties.confidence.
 *   Topic   — content = the topic phrase; properties.relevance,
 *             properties.keywords (string[]).
 *   Keyword — content = the keyword itself; no extra properties observed on
 *             disk, but rendered generically in case a future extractor adds
 *             some.
 * There is NO top-level `confidence` on text nodes (per textNodes.ts) — read
 * `properties.confidence`/`properties.relevance` instead, never the RawGraphNode
 * top-level field for text.
 *
 * Parent Document: derived by walking real `Contains` edges (the only real
 * text edge per graphRenderers/textEdges.ts) up from the node until a
 * `Document` node is reached — on-disk graphs show a direct Document->Entity/
 * Topic/Keyword edge, but the real edge set also allows an intervening
 * Section, so the walk isn't capped at one hop.
 */
import React, { useEffect, useMemo, useState } from "react";
import { loadGraphData } from "../../graphViewData";
import { GraphViewNode, GraphViewStatus } from "../../graphViewTypes";
import { navigateTo } from "../../navigation";

export type TextEntityBrowserProps = { projectId: number | null };

type Kind = "Entity" | "Topic" | "Keyword";
const KINDS: Kind[] = ["Entity", "Topic", "Keyword"];

const C_TEXT = "var(--color-text)";
const C_BODY = "var(--color-text-secondary)";
const C_MUTED = "var(--color-text-muted)";
const C_BORDER = "var(--color-border-faint)";

function numProp(node: GraphViewNode, key: string): number | undefined {
  const v = (node.raw.properties ?? {})[key];
  return typeof v === "number" ? v : undefined;
}
function strListProp(node: GraphViewNode, key: string): string[] {
  const v = (node.raw.properties ?? {})[key];
  return Array.isArray(v) ? v.filter((x): x is string => typeof x === "string") : [];
}
function strProp(node: GraphViewNode, key: string): string | undefined {
  const v = (node.raw.properties ?? {})[key];
  return typeof v === "string" ? v : undefined;
}
function otherProps(node: GraphViewNode, known: string[]): [string, unknown][] {
  return Object.entries(node.raw.properties ?? {}).filter(([k]) => !known.includes(k));
}

/** Nearest ancestor Document, walking real Contains edges (parent -> child). */
function findParentDocument(node: GraphViewNode, data: NonNullable<Extract<GraphViewStatus, { kind: "ready" }>["data"]>): GraphViewNode | undefined {
  const byId = new Map(data.nodes.map((n) => [n.id, n] as const));
  const incomingContains = new Map<string, string>(); // childId -> parentId
  for (const e of data.edges) {
    if (e.edgeType === "Contains") incomingContains.set(e.to, e.from);
  }
  let current: GraphViewNode | undefined = node;
  const seen = new Set<string>();
  while (current && !seen.has(current.id)) {
    seen.add(current.id);
    if (current.nodeType === "Document") return current;
    const parentId = incomingContains.get(current.id);
    current = parentId ? byId.get(parentId) : undefined;
  }
  return undefined;
}

const Card: React.FC<{ node: GraphViewNode; parentDoc: GraphViewNode | undefined; projectId: number }> = ({ node, parentDoc, projectId }) => {
  const confidence = numProp(node, "confidence");
  const relevance = numProp(node, "relevance");
  const entityType = strProp(node, "entity_type");
  const keywords = strListProp(node, "keywords");
  const known = ["confidence", "relevance", "entity_type", "keywords"];
  const rest = otherProps(node, known);

  return (
    <div
      style={{
        border: `1px solid ${C_BORDER}`,
        borderRadius: 8,
        padding: "8px 10px",
        marginBottom: 6,
        cursor: "pointer",
      }}
      onClick={() => navigateTo({ kind: "graph-node", projectId, nodeId: node.id })}
      title="Open in Graph View"
    >
      <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "baseline" }}>
        <span style={{ color: C_TEXT, fontWeight: 600, fontSize: 13 }}>{node.label}</span>
        <span style={{ fontSize: 11, color: C_MUTED }}>
          {confidence !== undefined && `confidence ${confidence.toFixed(2)}`}
          {relevance !== undefined && `relevance ${relevance.toFixed(2)}`}
        </span>
      </div>
      <div style={{ fontSize: 11.5, color: C_BODY, marginTop: 3 }}>
        {entityType && <span>type: {entityType} · </span>}
        {parentDoc ? <span>in: {parentDoc.label}</span> : <span style={{ color: C_MUTED }}>no Document ancestor found</span>}
      </div>
      {keywords.length > 0 && (
        <div style={{ fontSize: 11, color: C_MUTED, marginTop: 3 }}>keywords: {keywords.join(", ")}</div>
      )}
      {rest.length > 0 && (
        <div style={{ fontSize: 10.5, color: C_MUTED, marginTop: 3 }}>
          {rest.map(([k, v]) => `${k}: ${JSON.stringify(v)}`).join(" · ")}
        </div>
      )}
    </div>
  );
};

export const TextEntityBrowser: React.FC<TextEntityBrowserProps> = ({ projectId }) => {
  const [status, setStatus] = useState<GraphViewStatus>({ kind: "loading" });
  const [query, setQuery] = useState("");
  const [activeKind, setActiveKind] = useState<Kind | "all">("all");

  useEffect(() => {
    if (projectId === null) {
      setStatus({ kind: "empty" });
      return;
    }
    return loadGraphData(projectId, setStatus, { modality: "text" });
  }, [projectId]);

  const grouped = useMemo(() => {
    if (status.kind !== "ready") return { Entity: [], Topic: [], Keyword: [] } as Record<Kind, GraphViewNode[]>;
    const q = query.trim().toLowerCase();
    const out: Record<Kind, GraphViewNode[]> = { Entity: [], Topic: [], Keyword: [] };
    for (const n of status.data.nodes) {
      if (!KINDS.includes(n.nodeType as Kind)) continue;
      if (q && !n.label.toLowerCase().includes(q) && !(n.raw.content ?? "").toLowerCase().includes(q)) continue;
      out[n.nodeType as Kind].push(n);
    }
    return out;
  }, [status, query]);

  const total = grouped.Entity.length + grouped.Topic.length + grouped.Keyword.length;
  const kindsToShow: Kind[] = activeKind === "all" ? KINDS : [activeKind];

  if (projectId === null) {
    return <div style={{ padding: 16, color: C_MUTED, fontSize: 12.5 }}>Select a project to browse its text entities.</div>;
  }
  if (status.kind === "loading") {
    return <div style={{ padding: 16, color: C_MUTED, fontSize: 12.5 }}>Loading text graph…</div>;
  }
  if (status.kind === "error") {
    return <div style={{ padding: 16, color: "#ff8a8a", fontSize: 12.5 }}>Error: {status.message}</div>;
  }
  if (status.kind === "empty" || total === 0) {
    return (
      <div style={{ padding: 16, color: C_MUTED, fontSize: 12.5 }}>
        No text graph — or no Entity/Topic/Keyword nodes — exist for this project yet. Nothing fabricated to show in
        their place.
      </div>
    );
  }

  return (
    <div style={{ padding: 4 }}>
      <div style={{ display: "flex", gap: 10, alignItems: "center", marginBottom: 10, flexWrap: "wrap" }}>
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search entities, topics, keywords…"
          style={{
            background: "#101724",
            color: C_TEXT,
            border: `1px solid ${C_BORDER}`,
            borderRadius: 6,
            padding: "4px 8px",
            fontSize: 12,
            minWidth: 220,
          }}
        />
        <div style={{ display: "flex", gap: 4 }}>
          {(["all", ...KINDS] as const).map((k) => (
            <button
              key={k}
              onClick={() => setActiveKind(k)}
              style={{
                background: activeKind === k ? "var(--color-border-faint)" : "transparent",
                color: activeKind === k ? C_TEXT : C_MUTED,
                border: `1px solid ${C_BORDER}`,
                borderRadius: 6,
                padding: "3px 10px",
                fontSize: 11.5,
                cursor: "pointer",
              }}
            >
              {k === "all" ? `All (${total})` : `${k} (${grouped[k].length})`}
            </button>
          ))}
        </div>
      </div>

      {kindsToShow.map((kind) => {
        const items = grouped[kind];
        if (items.length === 0) return null;
        return (
          <div key={kind} style={{ marginBottom: 14 }}>
            {activeKind === "all" && (
              <div style={{ fontSize: 11, fontWeight: 700, letterSpacing: 0.4, textTransform: "uppercase", color: C_MUTED, margin: "4px 0 6px" }}>
                {kind} ({items.length})
              </div>
            )}
            {items.map((n) => (
              <Card key={n.id} node={n} parentDoc={status.kind === "ready" ? findParentDocument(n, status.data) : undefined} projectId={projectId} />
            ))}
          </div>
        );
      })}
    </div>
  );
};

export default TextEntityBrowser;
