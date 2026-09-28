/**
 * C9 — AMT lineage overlay (ForkOf / ContinuedBy), container-level edges.
 *
 * REAL SOURCES (verified this fork, not guessed):
 *  - src/orchestrator/amt.rs:156-190 — the FIRST AMT of a project is MAIN
 *    (keyword `amt-main`); every later generation is a FORK carrying the
 *    keyword `amt-fork-of:<prior_id>` and, since this session's fix, a real
 *    `ForkOf` Relation {target_id:prior, confidence:1.0, discovered_via:"Manual"}.
 *  - src/orchestrator/amt.rs:274-335 — the same write also puts a real
 *    `ContinuedBy` Relation into the PRIOR container (reverse edge).
 *  - src/types/container.rs:305-308 — RelationType::ForkOf = 60, ContinuedBy = 61.
 *  - Live host (read-only, project-parent container 3): 10 AMT generations
 *    (main 30411 + 9 forks). Only 40059 (ForkOf→30417) and 30417
 *    (ContinuedBy→40059) carry real Relation edges; the other 8 forks are
 *    KEYWORD-ONLY (pre-fix, one-directional, no confidence/discovered_via).
 *
 * HONESTY RULES enforced here:
 *  - Recorded edges pass the REAL Relation object through as `raw`
 *    (confidence / discovered_via exactly as stored).
 *  - Legacy keyword-derived ForkOf edges carry `{target_id, relation_type}`
 *    ONLY — confidence and discovered_via are absent, never invented — and
 *    render dotted so they never look like a verified edge.
 *  - A ContinuedBy edge is drawn ONLY when a real ContinuedBy Relation is
 *    recorded on the container. It is never synthesized as the inverse of a
 *    keyword link (the ForkOf edge already expresses that link).
 *
 * Response envelope: the host wraps /zsei/query results as
 * {success,result,error}; neither the Electron bridge nor the browser
 * fallback in ozoneClient.ts unwraps it, so this file unwraps locally
 * (works whether or not zseiQuery is later fixed centrally).
 */
import type { GraphOverlay, OverlayContext, OverlayResult } from "../overlays";
import type { EdgeVisual, NodeVisual } from "../types";
import type { ContainerRelation, GraphViewEdge, GraphViewNode } from "../../graphViewTypes";
import { zseiQuery } from "../../ozoneClient";
import { loadAmtGenerations, type AmtGeneration } from "../../data/amtLineage";

const LINEAGE_COLOR = "#b48cff";
const CONTINUED_COLOR = "#6fd3c8";
const FORK_KEYWORD = /^amt-fork-of:(\d+)$/;

// ── minimal shapes of the real GetContainer response ─────────────────────

interface ContainerJson {
  global_state?: { parent_id?: number };
  local_state?: {
    metadata?: { container_type?: string; name?: string | null };
    context?: { keywords?: string[]; relationships?: ContainerRelation[] };
  };
}

async function zsei<T>(query: Record<string, unknown>): Promise<T> {
  const r: any = await zseiQuery<any>(query);
  if (r && typeof r === "object" && "success" in r && "result" in r) {
    if (r.success === false || r.error) {
      throw new Error(typeof r.error === "string" ? r.error : "zsei query failed");
    }
    return r.result as T;
  }
  return r as T;
}

async function getContainer(id: number): Promise<ContainerJson> {
  const res = await zsei<any>({ GetContainer: { container_id: id } });
  const c = res && typeof res === "object" ? res.Container : undefined;
  if (!c) throw new Error(`container ${id} not returned`);
  return c as ContainerJson;
}

/** Bounded-concurrency map that never rejects: every item settles. */
async function settleAll<I, O>(items: I[], fn: (i: I) => Promise<O>, chunk = 8): Promise<PromiseSettledResult<O>[]> {
  const out: PromiseSettledResult<O>[] = [];
  for (let i = 0; i < items.length; i += chunk) {
    out.push(...(await Promise.allSettled(items.slice(i, i + chunk).map(fn))));
  }
  return out;
}

// ── hub + edge construction ──────────────────────────────────────────────

const hubId = (cid: number) => `hub:${cid}`;

function truncate(s: string, n: number): string {
  return s.length > n ? `${s.slice(0, n - 1)}…` : s;
}

function makeHub(
  cid: number,
  container: ContainerJson | null,
  gen: AmtGeneration | null,
  unresolvedReason?: string,
): GraphViewNode {
  const keywords = container?.local_state?.context?.keywords ?? gen?.keywords ?? [];
  const isMain = gen?.isMain ?? keywords.includes("amt-main");
  const name = gen?.name ?? container?.local_state?.metadata?.name ?? undefined;
  const nodeType = unresolvedReason ? "AmtUnresolved" : isMain ? "AmtMain" : "AmtFork";
  const tag = unresolvedReason ? "unreadable" : isMain ? "main" : "fork";
  const properties: Record<string, unknown> = { isMain, keywords };
  if (gen) properties.lineageSource = gen.lineageSource;
  if (container?.global_state?.parent_id !== undefined) properties.parentId = container.global_state.parent_id;
  if (name) properties.name = name;
  if (unresolvedReason) {
    properties.unresolved = true;
    properties.reason = unresolvedReason;
  }
  return {
    id: hubId(cid),
    modality: "container",
    hub: { kind: "amt", containerId: cid, title: name ?? `AMT ${cid}` },
    nodeType,
    label: `${tag} #${cid}${name ? ` · ${truncate(name, 40)}` : ""}`,
    contentPreview: name,
    // false only for a container we know solely by id from a real relation.
    isReal: !unresolvedReason,
    raw: { node_id: cid, node_type: nodeType, properties },
    sourceContainerId: cid,
  };
}

/** A relation is "recorded" when it carries the fields a real stored Relation
 * always has. The legacy keyword edge deliberately has neither. */
function isRecorded(raw: unknown): boolean {
  const r = raw as Partial<ContainerRelation> | null;
  return !!r && (typeof r.confidence === "number" || typeof r.discovered_via === "string");
}

function makeEdge(from: number, relation: ContainerRelation): GraphViewEdge {
  return {
    id: `lineage:${from}:${relation.target_id}:${relation.relation_type}`,
    from: hubId(from),
    to: hubId(relation.target_id),
    edgeType: relation.relation_type,
    edgeClass: "lineage",
    confidence: relation.confidence,
    discoveredVia: relation.discovered_via,
    isReal: true,
    raw: relation,
  };
}

// ── the overlay ──────────────────────────────────────────────────────────

async function load(ctx: OverlayContext): Promise<OverlayResult> {
  const seen = new Set<number>();
  const gens = (await loadAmtGenerations(ctx.projectId)).filter((g) => !seen.has(g.containerId) && seen.add(g.containerId));
  if (gens.length === 0) return { nodes: [], edges: [] };
  gens.sort((a, b) => a.containerId - b.containerId);

  // Read every generation's real container: recorded-vs-legacy is decided from
  // the stored relationships/keywords themselves, not inferred from a loader summary.
  const settled = await settleAll(gens, (g) => getContainer(g.containerId));
  const failed = settled.flatMap((s, i) => (s.status === "rejected" ? [`#${gens[i].containerId}: ${String((s as PromiseRejectedResult).reason?.message ?? (s as PromiseRejectedResult).reason)}`] : []));
  if (failed.length > 0) {
    throw new Error(`AMT lineage: could not read ${failed.length} of ${gens.length} AMT containers (${failed.slice(0, 2).join("; ")})`);
  }

  const nodes = new Map<string, GraphViewNode>();
  const edges = new Map<string, GraphViewEdge>();
  const genIds = new Set(gens.map((g) => g.containerId));

  gens.forEach((gen, i) => {
    const container = (settled[i] as PromiseFulfilledResult<ContainerJson>).value;
    nodes.set(hubId(gen.containerId), makeHub(gen.containerId, container, gen));

    const rels = container.local_state?.context?.relationships ?? [];
    let recordedForkParent: number | null = null;
    for (const r of rels) {
      if (r.relation_type !== "ForkOf" && r.relation_type !== "ContinuedBy") continue;
      const e = makeEdge(gen.containerId, r); // real Relation passed through untouched
      edges.set(e.id, e);
      if (r.relation_type === "ForkOf") recordedForkParent = r.target_id;
    }

    // Legacy mechanism: one-directional `amt-fork-of:<id>` keyword. Only drawn
    // when no recorded ForkOf already covers the same parent.
    const kwParent = (container.local_state?.context?.keywords ?? [])
      .map((k) => FORK_KEYWORD.exec(k))
      .find((m) => m)?.[1];
    const legacyParent = kwParent !== undefined ? Number(kwParent) : gen.forkOf;
    if (legacyParent !== null && legacyParent !== undefined && legacyParent !== recordedForkParent) {
      const e = makeEdge(gen.containerId, { target_id: legacyParent, relation_type: "ForkOf" });
      if (!edges.has(e.id)) edges.set(e.id, e);
    }
  });

  // Edge endpoints outside the loaded generations still need a node, or the
  // canvas silently drops the edge. Resolve them for real; if unreadable,
  // show a dashed not-real hub carrying the real error — never hide the link.
  const missing = Array.from(new Set(Array.from(edges.values()).map((e) => (e.raw as ContainerRelation).target_id))).filter((id) => !genIds.has(id));
  const resolved = await settleAll(missing, (id) => getContainer(id));
  missing.forEach((id, i) => {
    const s = resolved[i];
    nodes.set(
      hubId(id),
      s.status === "fulfilled"
        ? makeHub(id, s.value, null)
        : makeHub(id, null, null, String((s.reason as Error)?.message ?? s.reason)),
    );
  });

  return { nodes: Array.from(nodes.values()), edges: Array.from(edges.values()) };
}

function nodeVisual(node: GraphViewNode): NodeVisual {
  switch (node.nodeType) {
    case "AmtMain":
      return { shape: "hexagon", radius: 11, fill: LINEAGE_COLOR, stroke: "#e6d8ff", glyph: "M" };
    case "AmtFork":
      return { shape: "diamond", radius: 9, fill: "#8f6fe0", glyph: "F" };
    default:
      // AmtUnresolved (or anything unexpected): visibly not-real, never styled as a live generation.
      return { shape: "diamond", radius: 8, fill: "#9aa5b5", opacity: 0.55, strokeDasharray: "3 2", stroke: "#9aa5b5" };
  }
}

function edgeVisual(edge: GraphViewEdge): EdgeVisual {
  const recorded = isRecorded(edge.raw);
  if (edge.edgeType === "ContinuedBy") {
    // Only ever drawn from a recorded relation.
    return { stroke: CONTINUED_COLOR, strokeWidth: 2, arrow: true };
  }
  return recorded
    ? { stroke: LINEAGE_COLOR, strokeWidth: 2, arrow: true }
    : // legacy keyword-only link: dotted + dimmer, so it never reads as a verified edge
      { stroke: LINEAGE_COLOR, strokeWidth: 1.5, strokeDasharray: "2 4", opacity: 0.75, arrow: true };
}

export const lineageOverlay: GraphOverlay = {
  id: "lineage",
  label: "AMT lineage",
  description: "ForkOf / ContinuedBy links between AMT generations (dotted = legacy keyword-only, no recorded relation)",
  edgeClass: "lineage",
  load,
  nodeVisual,
  edgeVisual,
};
