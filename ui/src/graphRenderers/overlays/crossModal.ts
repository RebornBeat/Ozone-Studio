/**
 * C8 — Cross-modal `SimilarTo` overlay.
 *
 * Real sources (verified, not assumed):
 *  - Creation: each modality pipeline's `link_related_containers`
 *    (assets/pipelines/modalities/{code,math,text}/main.rs — code:375/580, math:2350/2555,
 *    text:228/444) keyword-searches ALL containers, walks the relationship neighborhood, and writes a
 *    bidirectional `SimilarTo` Relation {target_id, relation_type:"SimilarTo", confidence,
 *    discovered_via:"CodeAnalysis"|"TextAnalysis"|..., graph_hops?} into `Context.relationships` of BOTH
 *    containers. The match is purely lexical (docs/GRAPH_RELATIONSHIP_REGISTRY.md §5), so a target is NOT
 *    necessarily a graph of another modality — it can be same-modality, or a non-graph container.
 *  - Read path: `graphData.containerRelations[graphContainerId]` (B16 — already fetched with each
 *    ModalityGraph container, no extra call) + `GetContainer` on each endpoint to describe it truthfully.
 *
 * Live measurement (host :50051, 28 ModalityGraph containers): 158 SimilarTo relations — 29 truly
 * cross-modal (code→math 4, code→text 10, text→code 10, math→code 5), 128 same-modality (code→code 120,
 * text→text 8), 1 to a non-graph `Derived` container; 75 symmetric A→B/B→A pairs; 0 self-loops.
 * Every relation is kept as stored (no de-duplication of symmetric pairs). Cross-modal links are drawn
 * heavy and bright, same-modality and non-graph links lighter and dashed, so the overlay does not
 * overstate how many links really cross modalities.
 */
import { zseiQuery } from "../../ozoneClient";
import type { ContainerRelation, GraphViewEdge, GraphViewNode, HubKind, Modality } from "../../graphViewTypes";
import type { GraphOverlay, OverlayContext, OverlayResult } from "../overlays";
import { MODALITY_COLOR } from "../defaults";
import type { EdgeVisual, NodeVisual } from "../types";

interface ContainerLike {
  global_state?: { parent_id?: number; child_ids?: number[]; child_count?: number };
  local_state?: {
    metadata?: { container_type?: string; modality?: string; name?: string | null };
    context?: { keywords?: string[]; relationships?: ContainerRelation[] };
    storage?: { object_store_path?: string | null };
  };
}

/** The host (and the Electron bridge, which forwards the host's JSON verbatim) answers
 * `{success, result, error}`; unwrap it, and tolerate an already-unwrapped value. */
function unwrap(resp: unknown): { ok: boolean; value: any; error?: string } {
  if (resp && typeof resp === "object" && "success" in (resp as object) && "result" in (resp as object)) {
    const r = resp as { success: boolean; result: unknown; error?: string | null };
    return { ok: r.success, value: r.result, error: r.error ?? undefined };
  }
  return { ok: true, value: resp };
}

/** null = the container genuinely does not exist (deleted target); any other failure throws. */
async function fetchContainer(id: number): Promise<ContainerLike | null> {
  const { ok, value, error } = unwrap(await zseiQuery<unknown>({ GetContainer: { container_id: id } }));
  if (!ok) {
    if (error && /not found/i.test(error)) return null;
    throw new Error(`GetContainer ${id} failed: ${error ?? "unknown error"}`);
  }
  return (value && typeof value === "object" && "Container" in value ? (value as any).Container : null) as ContainerLike | null;
}

function modalityFromPath(path: string | null | undefined): Modality | null {
  if (!path) return null;
  if (path.startsWith("graphs/code_")) return "code";
  if (path.startsWith("graphs/math_")) return "math";
  if (path.startsWith("graphs/text_")) return "text";
  return null;
}

function isAmtContainer(c: ContainerLike): boolean {
  const kws = c.local_state?.context?.keywords ?? [];
  return c.local_state?.metadata?.container_type === "Derived" && kws.some((k) => k === "amt-main" || k.startsWith("amt-fork-of:"));
}

interface Described {
  id: number;
  found: boolean;
  containerType: string;
  modality: Modality | null;
  hubKind: HubKind;
  title: string;
  properties: Record<string, unknown>;
}

function describe(id: number, c: ContainerLike | null): Described {
  if (!c) {
    return {
      id,
      found: false,
      containerType: "Unknown",
      modality: null,
      hubKind: "modality-graph",
      title: `Container ${id} (not found)`,
      properties: { container_id: id, exists: false },
    };
  }
  const containerType = c.local_state?.metadata?.container_type ?? "Unknown";
  const path = c.local_state?.storage?.object_store_path ?? null;
  const modality = containerType === "ModalityGraph" ? modalityFromPath(path) : null;
  const name = c.local_state?.metadata?.name ?? null;
  const amt = isAmtContainer(c);
  const label = containerType === "ModalityGraph" ? `${modality ?? "unknown"} graph` : amt ? "AMT generation" : containerType;
  const properties: Record<string, unknown> = { container_id: id, container_type: containerType };
  if (modality) properties.modality = modality;
  if (name) properties.name = name;
  if (path) properties.object_store_path = path;
  if (c.global_state?.parent_id !== undefined) properties.parent_id = c.global_state.parent_id;
  return {
    id,
    found: true,
    containerType,
    modality,
    hubKind: amt ? "amt" : "modality-graph",
    title: name ? `${label} #${id} — ${name}` : `${label} #${id}`,
    properties,
  };
}

function hubNode(d: Described): GraphViewNode {
  return {
    id: `hub:${d.id}`,
    modality: "container",
    hub: { kind: d.hubKind, containerId: d.id, title: d.title },
    nodeType: d.containerType,
    label: d.title,
    isReal: d.found,
    raw: { node_id: d.id, node_type: d.containerType, properties: d.properties },
    sourceContainerId: d.id,
  };
}

/** relation kind per edge id, filled by load(); edgeVisual() only receives the edge. */
const linkKind = new Map<string, "cross" | "same" | "other">();

async function load(ctx: OverlayContext): Promise<OverlayResult> {
  const relations: { src: number; rel: ContainerRelation }[] = [];
  for (const [srcKey, rels] of Object.entries(ctx.graphData.containerRelations)) {
    const src = Number(srcKey);
    for (const rel of rels) {
      // A self-loop cannot be drawn between two hubs (an earlier pipeline bug wrote some; none exist live).
      if (rel.relation_type === "SimilarTo" && rel.target_id !== src) relations.push({ src, rel });
    }
  }
  if (relations.length === 0) return { nodes: [], edges: [] };

  const ids = Array.from(new Set(relations.flatMap(({ src, rel }) => [src, rel.target_id])));
  const described = new Map<number, Described>();
  for (let i = 0; i < ids.length; i += 8) {
    const chunk = ids.slice(i, i + 8);
    const containers = await Promise.all(chunk.map((id) => fetchContainer(id)));
    chunk.forEach((id, j) => described.set(id, describe(id, containers[j])));
  }

  // The loaded graph nodes already know each source graph's modality; prefer that over re-deriving it.
  const knownModality = new Map<number, Modality>();
  for (const n of ctx.graphData.nodes) if (n.modality !== "container") knownModality.set(n.sourceContainerId, n.modality);

  const nodes = new Map<string, GraphViewNode>();
  const edges: GraphViewEdge[] = [];
  linkKind.clear();
  relations.sort((a, b) => a.src - b.src || a.rel.target_id - b.rel.target_id);
  for (const { src, rel } of relations) {
    const s = described.get(src)!;
    const t = described.get(rel.target_id)!;
    const sMod = knownModality.get(src) ?? s.modality;
    const tMod = knownModality.get(rel.target_id) ?? t.modality;
    const kind = sMod && tMod ? (sMod === tMod ? "same" : "cross") : "other";
    nodes.set(`hub:${src}`, hubNode(s));
    nodes.set(`hub:${rel.target_id}`, hubNode(t));
    const id = `xmodal:${src}:${rel.target_id}:${rel.relation_type}`;
    linkKind.set(id, kind);
    edges.push({
      id,
      from: `hub:${src}`,
      to: `hub:${rel.target_id}`,
      edgeType: "SimilarTo",
      edgeClass: "cross-modal",
      confidence: rel.confidence,
      discoveredVia: rel.discovered_via,
      isReal: true,
      raw: rel,
    });
  }
  return { nodes: Array.from(nodes.values()), edges };
}

const NEUTRAL = "#9aa5b5";

function nodeVisual(node: GraphViewNode): NodeVisual {
  const props = (node.raw.properties ?? {}) as Record<string, unknown>;
  const modality = typeof props.modality === "string" ? (props.modality as Modality) : null;
  if (node.hub?.kind === "modality-graph" && node.nodeType === "ModalityGraph" && modality && modality in MODALITY_COLOR) {
    return { shape: "square", radius: 10, fill: MODALITY_COLOR[modality], glyph: modality[0].toUpperCase() };
  }
  // A real container that is not a modality graph (or a graph whose modality can't be read): neutral, not dimmed
  // — it exists — but a different shape so it is never mistaken for a text/code/math graph.
  return {
    shape: "diamond",
    radius: 10,
    fill: NEUTRAL,
    glyph: node.nodeType && node.nodeType !== "Unknown" ? node.nodeType[0].toUpperCase() : "?",
    opacity: node.isReal ? 1 : 0.5,
    strokeDasharray: node.isReal ? undefined : "3 2",
  };
}

function edgeVisual(edge: GraphViewEdge): EdgeVisual {
  switch (linkKind.get(edge.id)) {
    case "cross":
      return { stroke: "#e879f9", strokeWidth: 3, opacity: 0.95 };
    case "same":
      return { stroke: "#5b6b86", strokeWidth: 1.5, strokeDasharray: "5 4", opacity: 0.75 };
    default:
      return { stroke: NEUTRAL, strokeWidth: 2, strokeDasharray: "2 3", opacity: 0.8 };
  }
}

export const crossModalOverlay: GraphOverlay = {
  id: "cross-modal",
  label: "Cross-modal similarity",
  description:
    "SimilarTo links (lexical keyword overlap, written by the modality pipelines) from this project's graphs to other containers — heavy magenta = across modalities, dashed = same modality or non-graph",
  edgeClass: "cross-modal",
  load,
  nodeVisual,
  edgeVisual,
};
