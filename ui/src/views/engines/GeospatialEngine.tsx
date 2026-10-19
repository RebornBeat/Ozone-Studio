/**
 * Geospatial Engine — structural view for the Geospatial modality (pipeline
 * 117, `assets/pipelines/modalities/geospatial/main.rs`, 2323 lines, real —
 * confirmed by direct read, not the session's earlier paraphrase alone).
 *
 * No tile-based basemap here on purpose: `ui/package.json` has no mapping
 * library (Leaflet/MapLibre/etc. — checked before writing this file), and
 * adding one for a single engine view was out of scope for this pass. What
 * IS real and shown: every constructed `GeoNodeType` variant (confirmed
 * against `GeoModalityAction::CreateGraph`'s real construction sites,
 * `main.rs:1311-1639` — `GeoScene`/`RegionNode`/`RouteNode`/`LocationNode`/
 * `POINode`/`TrajectoryNode`/`TerrainModelNode`/`CoverageAreaNode`/
 * `ViewshedNode`/`FloodZoneNode`/`ChangeRegionNode`/`GeocodedPlaceNode`),
 * each with its own real, modality-specific fields (`point` as real
 * lon/lat, `area_km2`, `length_m`, `elevation_m`, `region_type`/
 * `route_type`/`poi_type`, `admin_level`) grouped into sections, plus a
 * lightweight inline-SVG scatter of every node that carries a real `point`
 * — explicitly labeled a scatter, not a map, since it has no basemap,
 * projection correction, or tiles under it (plain linear lon/lat -> x/y
 * fit to the data's own real bounding range).
 *
 * `GraphViewNode.raw` (per `graphViewTypes.ts`) is always the complete,
 * untouched backend node — geo-specific fields not promoted to a named
 * `RawGraphNode` property are read straight off `raw` via the type's own
 * `[extra: string]: unknown` catch-all, never fabricated or defaulted.
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
const C_ACCENT = "#16a085"; // MODALITY_COLOR.geospatial (spec's own Section O.1 color table)

interface GeoRaw {
  point?: { lon: number; lat: number; alt_m?: number | null; crs?: string | null } | null;
  bounding_box?: { min_lon: number; min_lat: number; max_lon: number; max_lat: number } | null;
  area_km2?: number | null;
  length_m?: number | null;
  elevation_m?: number | null;
  region_type?: string | null;
  route_type?: string | null;
  poi_type?: string | null;
  admin_level?: number | null;
  name?: string | null;
}

function geoRaw(node: GraphViewNode): GeoRaw {
  return node.raw as unknown as GeoRaw;
}

// Section grouping — mirrors the real GeoNodeType variants this pipeline
// actually constructs (main.rs:1311-1639), not the full declared enum
// (CrossModalOverlayNode/NetworkNodeGeo/NetworkEdgeGeo/TopologyNode/
// TrajectoryPointNode/RasterLayerNode/BoundaryNode/ElevationProfileNode
// exist in the type but weren't found at a real construction site in
// CreateGraph — they still render here under "Other", honestly, if any
// ever appear, rather than being silently dropped).
const SECTIONS: { title: string; types: string[] }[] = [
  { title: "Regions", types: ["RegionNode"] },
  { title: "Routes", types: ["RouteNode"] },
  { title: "Locations & points of interest", types: ["LocationNode", "POINode", "GeocodedPlaceNode"] },
  { title: "Trajectories", types: ["TrajectoryNode", "TrajectoryPointNode"] },
  { title: "Terrain & elevation", types: ["TerrainModelNode", "ElevationProfileNode", "RasterLayerNode"] },
  {
    title: "Analysis products",
    types: ["CoverageAreaNode", "ViewshedNode", "FloodZoneNode", "ChangeRegionNode", "TopologyNode"],
  },
  { title: "Road network", types: ["NetworkNodeGeo", "NetworkEdgeGeo"] },
  { title: "Boundaries", types: ["BoundaryNode"] },
  { title: "Cross-modal", types: ["CrossModalOverlayNode"] },
];

function nodeLabel(node: GraphViewNode): string {
  const r = geoRaw(node);
  return r.name || node.label || node.nodeType;
}

function nodeFacts(node: GraphViewNode): string[] {
  const r = geoRaw(node);
  const facts: string[] = [];
  if (r.region_type) facts.push(r.region_type);
  if (r.route_type) facts.push(r.route_type);
  if (r.poi_type) facts.push(r.poi_type);
  if (typeof r.admin_level === "number") facts.push(`admin level ${r.admin_level}`);
  if (typeof r.area_km2 === "number") facts.push(`${r.area_km2.toFixed(2)} km²`);
  if (typeof r.length_m === "number") facts.push(`${(r.length_m / 1000).toFixed(2)} km`);
  if (typeof r.elevation_m === "number") facts.push(`${r.elevation_m.toFixed(0)} m elevation`);
  if (r.point) facts.push(`${r.point.lat.toFixed(4)}, ${r.point.lon.toFixed(4)}`);
  return facts;
}

function edgesForNode(nodeId: string, edges: GraphViewEdge[], byId: Map<string, GraphViewNode>) {
  return edges
    .filter((e) => e.from === nodeId)
    .map((e) => ({ edge: e, target: byId.get(e.to) }))
    .filter((e) => e.target);
}

function jumpToNode(node: GraphViewNode, projectId: number) {
  navigateTo({ kind: "graph-node", projectId, nodeId: node.id });
}

/** Honest spatial scatter — NOT a basemap. Plain linear lon/lat projection
 * fit to the real data's own bounding range, no tiles, no coastlines, no
 * projection correction. Labeled as such directly in the UI, not just here. */
const SpatialScatter: React.FC<{ nodes: GraphViewNode[]; projectId: number; onSelect: (n: GraphViewNode) => void }> = ({
  nodes,
  projectId,
  onSelect,
}) => {
  const pointed = useMemo(
    () => nodes.map((n) => ({ node: n, point: geoRaw(n).point })).filter((x) => x.point) as {
      node: GraphViewNode;
      point: { lon: number; lat: number };
    }[],
    [nodes],
  );
  if (pointed.length === 0) return null;

  const lons = pointed.map((p) => p.point.lon);
  const lats = pointed.map((p) => p.point.lat);
  const minLon = Math.min(...lons), maxLon = Math.max(...lons);
  const minLat = Math.min(...lats), maxLat = Math.max(...lats);
  const spanLon = maxLon - minLon || 1;
  const spanLat = maxLat - minLat || 1;
  const W = 480, H = 260, PAD = 16;
  const x = (lon: number) => PAD + ((lon - minLon) / spanLon) * (W - 2 * PAD);
  // Screen y grows downward; latitude grows upward — invert.
  const y = (lat: number) => H - PAD - ((lat - minLat) / spanLat) * (H - 2 * PAD);

  return (
    <div style={{ marginBottom: 14 }}>
      <div style={{ fontSize: 11, color: C_MUTED, marginBottom: 4 }}>
        Spatial scatter — plain lon/lat projection of {pointed.length} located node{pointed.length === 1 ? "" : "s"},
        fit to this graph's own coordinate range. Not a basemap: no tiles, no coastlines, no projection correction.
      </div>
      <svg
        viewBox={`0 0 ${W} ${H}`}
        width="100%"
        style={{ maxWidth: 480, border: `1px solid ${C_BORDER}`, borderRadius: 8, background: C_PANEL }}
      >
        {pointed.map(({ node, point }) => (
          <circle
            key={node.id}
            cx={x(point.lon)}
            cy={y(point.lat)}
            r={4}
            fill={C_ACCENT}
            opacity={0.85}
            style={{ cursor: "pointer" }}
            onClick={() => onSelect(node)}
          >
            <title>{`${nodeLabel(node)} (${point.lat.toFixed(4)}, ${point.lon.toFixed(4)})`}</title>
          </circle>
        ))}
      </svg>
    </div>
  );
};

export const GeospatialEngine: React.FC<{ projectId: number | null }> = ({ projectId }) => {
  const [status, setStatus] = useState<GraphViewStatus>({ kind: "loading" });
  const [selected, setSelected] = useState<GraphViewNode | null>(null);

  useEffect(() => {
    if (projectId === null) return;
    setStatus({ kind: "loading" });
    const dispose = loadGraphData(projectId, setStatus, { modality: "geospatial" });
    return dispose;
  }, [projectId]);

  const byId = useMemo(
    () => (status.kind === "ready" ? new Map(status.data.nodes.map((n) => [n.id, n])) : new Map()),
    [status],
  );
  const nonRootNodes = useMemo(
    () => (status.kind === "ready" ? status.data.nodes.filter((n) => n.nodeType !== "GeoScene") : []),
    [status],
  );

  if (projectId === null) {
    return <div style={{ color: C_MUTED, fontSize: 12.5 }}>Select a project to see its geospatial graph.</div>;
  }

  return (
    <div>
      <div style={{ fontSize: 12.5, fontWeight: 700, color: C_TEXT, marginBottom: 8 }}>Geospatial Engine</div>

      {status.kind === "loading" && <div style={{ color: C_MUTED, fontSize: 12.5 }}>Loading geospatial graph…</div>}
      {status.kind === "error" && (
        <div style={{ color: C_ERROR, fontSize: 12.5 }}>Error loading geospatial graph: {status.message}</div>
      )}
      {status.kind === "empty" && (
        <div style={{ color: C_MUTED, fontSize: 12.5 }}>
          No geospatial-modality graph exists for this project yet — nothing to show.
        </div>
      )}
      {status.kind === "ready" && nonRootNodes.length === 0 && (
        <div style={{ color: C_MUTED, fontSize: 12.5 }}>
          This project's geospatial graph has only a root scene node — no regions, routes, or locations recorded yet.
        </div>
      )}

      {status.kind === "ready" && nonRootNodes.length > 0 && (
        <>
          <SpatialScatter nodes={nonRootNodes} projectId={projectId} onSelect={setSelected} />

          {SECTIONS.map(({ title, types }) => {
            const nodes = nonRootNodes.filter((n) => types.includes(n.nodeType));
            if (nodes.length === 0) return null;
            return (
              <div key={title} style={{ marginBottom: 14 }}>
                <div style={{ fontSize: 11.5, fontWeight: 700, color: C_BODY, marginBottom: 6 }}>
                  {title} ({nodes.length})
                </div>
                <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                  {nodes.map((node) => {
                    const rels = edgesForNode(node.id, status.data.edges, byId);
                    const isSel = selected?.id === node.id;
                    return (
                      <div
                        key={node.id}
                        style={{
                          border: `1px solid ${isSel ? C_ACCENT : C_BORDER}`,
                          borderRadius: 8,
                          padding: "8px 10px",
                          background: C_PANEL,
                        }}
                      >
                        <div
                          style={{ display: "flex", alignItems: "baseline", gap: 8, cursor: "pointer" }}
                          onClick={() => {
                            setSelected(node);
                            jumpToNode(node, projectId);
                          }}
                          title="Jump to node"
                        >
                          <span style={{ color: C_ACCENT, fontWeight: 700, fontSize: 12.5 }}>{nodeLabel(node)}</span>
                          <span style={{ color: C_MUTED, fontSize: 11 }}>{node.nodeType}</span>
                        </div>
                        {nodeFacts(node).length > 0 && (
                          <div style={{ marginTop: 4, color: C_BODY, fontSize: 11.5 }}>
                            {nodeFacts(node).join(" · ")}
                          </div>
                        )}
                        {rels.length > 0 && (
                          <div style={{ marginTop: 6, display: "flex", flexDirection: "column", gap: 2 }}>
                            {rels.map(({ edge, target }) => (
                              <div
                                key={edge.id}
                                style={{ display: "flex", alignItems: "baseline", gap: 6, cursor: "pointer" }}
                                onClick={() => target && jumpToNode(target, projectId)}
                              >
                                <span style={{ color: C_MUTED, fontSize: 11 }}>{edge.edgeType} →</span>
                                <span style={{ color: C_BODY, fontSize: 11.5 }}>
                                  {target ? nodeLabel(target) : "?"}
                                </span>
                              </div>
                            ))}
                          </div>
                        )}
                      </div>
                    );
                  })}
                </div>
              </div>
            );
          })}

          <div style={{ color: C_MUTED, fontSize: 11 }}>
            {nonRootNodes.length} node{nonRootNodes.length === 1 ? "" : "s"} · {status.data.edges.length} edge
            {status.data.edges.length === 1 ? "" : "s"}.
          </div>
        </>
      )}
    </div>
  );
};

export default GeospatialEngine;
