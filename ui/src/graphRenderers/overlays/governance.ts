/**
 * C10 — Jurisdiction overlay: `RelatedTo` links between jurisdiction scopes.
 *
 * Jurisdiction is global (not per-project), so `load()` ignores `projectId`.
 * Each real `JurisdictionRuleSet` container under JURISDICTION_ROOT_ID (7)
 * becomes one hub node; each stored `RelatedTo` relation whose target is ALSO a
 * real rule set becomes a governance edge (national scope -> EU/global
 * baseline, src/lib.rs ~651-690). Data comes from data/jurisdictionData.ts
 * (B14), which documents the live-host findings.
 *
 * Not drawn, deliberately: relations whose target is a blank `Root`-type
 * placeholder rather than a rule set (204 of 263 in the live data) — a hub for
 * a nameless placeholder would misrepresent it as a scope. They are NOT lost:
 * each source hub carries `properties.unresolvedRelationCount` /
 * `unresolvedTargetIds`, which the node detail panel (C11) shows in its
 * properties view.
 *
 * Relation types are passed through untouched. `RelatedTo` is the only type the
 * pipeline constructs for jurisdiction; anything else (e.g. the declared-but-
 * unwired `JurisdictionScope` type) is flagged `isReal: false` if it ever appears.
 */
import type { ContainerRelation, GraphViewEdge, GraphViewNode, RawGraphNode } from "../../graphViewTypes";
import type { GraphOverlay, OverlayResult } from "../overlays";
import type { EdgeVisual, NodeVisual } from "../types";
import { loadJurisdictionGraph, type JurisdictionScope } from "../../data/jurisdictionData";

const GOVERNANCE_COLOR = "#4fd1c5";
/** Global/EU are the baseline scopes every other scope layers on (registry: national -> eu -> global). */
const BASELINE_GLYPH: Record<string, string> = { global: "G", eu: "E" };

function hubTitle(s: JurisdictionScope): string {
  // Two containers can share a region key (bootstrap + search-retrieved) — disambiguate by id.
  return s.regionKeyShared ? `${s.regionKey} #${s.containerId}` : s.regionKey;
}

async function load(): Promise<OverlayResult> {
  const graph = await loadJurisdictionGraph();
  const byId = new Map(graph.scopes.map((s) => [s.containerId, s]));

  const nodes: GraphViewNode[] = graph.scopes.map((s) => {
    const raw: RawGraphNode = {
      node_id: s.containerId,
      node_type: "JurisdictionRuleSet",
      label: s.name,
      properties: {
        regionKey: s.regionKey,
        regionKeyShared: s.regionKeyShared,
        ...(s.provenance ? { provenance: s.provenance } : {}),
        ...(s.objectStorePath ? { objectStorePath: s.objectStorePath } : {}),
        outgoingRelationCount: s.relations.length,
        unresolvedRelationCount: s.unresolvedRelationCount,
        ...(s.unresolvedRelationCount > 0
          ? { unresolvedTargetIds: s.relations.filter((r) => !r.targetResolved).map((r) => r.targetId) }
          : {}),
      },
    };
    return {
      id: `hub:${s.containerId}`,
      modality: "container",
      hub: { kind: "jurisdiction", containerId: s.containerId, title: hubTitle(s) },
      nodeType: "JurisdictionRuleSet",
      label: hubTitle(s),
      contentPreview: s.name,
      isReal: true,
      raw,
      sourceContainerId: s.containerId,
    };
  });

  const edges: GraphViewEdge[] = [];
  const seen = new Map<string, number>();
  for (const s of graph.scopes) {
    for (const r of s.relations) {
      if (!r.targetResolved || !byId.has(r.targetId)) continue;
      const base = `gov:${s.containerId}:${r.targetId}:${r.relationType}`;
      const n = (seen.get(base) ?? 0) + 1;
      seen.set(base, n);
      edges.push({
        id: n === 1 ? base : `${base}#${n}`,
        from: `hub:${s.containerId}`,
        to: `hub:${r.targetId}`,
        edgeType: r.relationType,
        edgeClass: "governance",
        ...(r.confidenceRecorded ? { confidence: r.confidence } : {}),
        ...(r.discoveryRecorded ? { discoveredVia: r.discoveredVia } : {}),
        isReal: r.relationType === "RelatedTo",
        raw: r.raw as ContainerRelation,
      });
    }
  }
  return { nodes, edges };
}

function nodeVisual(node: GraphViewNode): NodeVisual {
  const key = String((node.raw.properties as Record<string, unknown>)?.regionKey ?? "");
  const glyph = BASELINE_GLYPH[key];
  return {
    shape: "triangle",
    radius: glyph ? 11 : 8,
    fill: GOVERNANCE_COLOR,
    stroke: "#0a0f1a",
    strokeWidth: 1,
    glyph,
    // A region key with two containers is real but ambiguous — mark it, don't merge it.
    ...((node.raw.properties as Record<string, unknown>)?.regionKeyShared ? { strokeDasharray: "2 2", stroke: "#e8fffb" } : {}),
  };
}

function edgeVisual(edge: GraphViewEdge): EdgeVisual {
  if (!edge.isReal) return { stroke: "#6b7a90", strokeWidth: 1, strokeDasharray: "2 3", opacity: 0.5, arrow: true };
  return { stroke: GOVERNANCE_COLOR, strokeWidth: 1, opacity: 0.55, arrow: true };
}

export const governanceOverlay: GraphOverlay = {
  id: "governance",
  label: "Jurisdiction",
  description: "RelatedTo links between jurisdiction scopes (national → EU/global baseline)",
  edgeClass: "governance",
  load,
  nodeVisual,
  edgeVisual,
};
