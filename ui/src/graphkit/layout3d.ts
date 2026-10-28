/**
 * GraphKit — 3D world layout + lens highlighting (pure functions, no React,
 * no three.js). The world is the SUBSTRATE drawn in space:
 *
 *   project (root) ── graph hubs (one per ModalityGraph container, on a
 *   sphere around the root) ── that graph's nodes (a cluster around its hub)
 *   files (outer ring, pulled toward the graphs they feed)
 *   other containers (methodologies, blueprints, AMTs, tasks… outer shell,
 *   grouped by type)
 *   edges: in-graph edges, root→hub containment, container-level cross-links
 *   (cross-modal SimilarTo drawn in magenta), file→graph links.
 *
 * Deterministic: the same substrate always lays out the same way (golden-angle
 * Fibonacci spheres, stable sort keys), so the world never reshuffles under the
 * operator when a live event refetches the substrate. Honest: when a cap trims
 * the drawn set, the layout reports exactly how many of how many are shown.
 */
import { MODALITY_COLOR } from "../graphRenderers/defaults";
import type { GraphViewEdge, GraphViewNode, Modality } from "../graphViewTypes";
import type { GKContainerRef, GKCrossLink, GKFileRef } from "./types";

export type WorldKind = "root" | "hub" | "node" | "file" | "container";
export type WorldShape = "sphere" | "box" | "octa" | "ico";
export type WorldEdgeClass =
  | "structural"
  | "dependency"
  | "semantic"
  | "cross-modal"
  | "lineage"
  | "governance"
  | "link" // container-level link that is not cross-modal similarity
  | "file" // file → graph it feeds
  | "root"; // project → graph hub containment

export interface WorldNode {
  index: number;
  id: string;
  kind: WorldKind;
  pos: [number, number, number];
  color: string;
  label: string;
  nodeType: string;
  modality: string;
  radius: number;
  shape: WorldShape;
  isReal: boolean;
  /** The container this entity belongs to / is (hub: the graph container;
   * node: its source graph container; file/container: itself; root: project). */
  containerId: number | null;
  degree: number;
  /** Hubs only: radius of the cluster shell around the hub (0 elsewhere) —
   * labels sit above the shell, not on top of the cluster. */
  shell: number;
}

export interface WorldEdge {
  a: number; // WorldNode.index
  b: number;
  cls: WorldEdgeClass;
}

export interface WorldTruncation {
  graphNodes: { shown: number; total: number };
  containers: { shown: number; total: number };
  files: { shown: number; total: number };
}

export interface WorldLayout {
  nodes: WorldNode[];
  edges: WorldEdge[];
  byId: Map<string, number>;
  /** graph-container id → hub index */
  hubByContainer: Map<number, number>;
  /** any container id (hub, file, other container) → world index */
  entityByContainer: Map<number, number>;
  /** bounding radius of the whole world (camera framing) */
  radius: number;
  truncation: WorldTruncation;
  /** modality → number of graph nodes drawn (legend counts) */
  modalityCounts: Record<string, number>;
}

export const MAX_GRAPH_NODES = 2400;
export const MAX_CONTAINER_NODES = 300;
export const MAX_FILE_NODES = 300;

const GOLDEN = Math.PI * (3 - Math.sqrt(5));
const C_ROOT = "#e6edf3";
const C_FILE = "#4fc3f7";
const CONTAINER_PALETTE = [
  "#8da2fb", "#f59e0b", "#34d399", "#f472b6", "#a78bfa",
  "#fb7185", "#22d3ee", "#c4b5fd", "#fbbf24", "#6ee7b7",
];

function fib(i: number, n: number): [number, number, number] {
  const y = n <= 1 ? 0 : 1 - (i / (n - 1)) * 2;
  const r = Math.sqrt(Math.max(0, 1 - y * y));
  const t = GOLDEN * i;
  return [Math.cos(t) * r, y, Math.sin(t) * r];
}

function hashStr(s: string): number {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

function shapeFor(modality: string): WorldShape {
  switch (modality) {
    case "code":
    case "math":
      return "ico";
    case "text":
    case "image":
      return "box";
    case "3D":
    case "CAD":
    case "geospatial":
      return "octa";
    default:
      return "sphere";
  }
}

function clip(s: string, n: number): string {
  return s.length > n ? s.slice(0, n - 1) + "…" : s;
}

export function computeWorldLayout(input: {
  projectId: number;
  projectName: string | null;
  nodes: GraphViewNode[];
  edges: GraphViewEdge[];
  graphContainers: Record<number, { modality: Modality; name: string | null }>;
  files: GKFileRef[];
  containers: Record<number, GKContainerRef>;
  crossLinks: GKCrossLink[];
}): WorldLayout {
  const { projectId, projectName, graphContainers, files, containers, crossLinks } = input;

  const nodes: WorldNode[] = [];
  const byId = new Map<string, number>();
  const hubByContainer = new Map<number, number>();
  const entityByContainer = new Map<number, number>();
  const push = (n: Omit<WorldNode, "index" | "degree" | "shell"> & { shell?: number }): number => {
    const index = nodes.length;
    nodes.push({ ...n, shell: n.shell ?? 0, index, degree: 0 });
    byId.set(n.id, index);
    return index;
  };

  // Root: the project itself.
  const rootIdx = push({
    id: `root:${projectId}`,
    kind: "root",
    pos: [0, 0, 0],
    color: C_ROOT,
    label: projectName ?? `Project #${projectId}`,
    nodeType: "Project",
    modality: "container",
    radius: 1.5,
    shape: "ico",
    isReal: true,
    containerId: projectId,
  });
  entityByContainer.set(projectId, rootIdx);

  // Split the substrate's nodes: graph nodes (belong to a ModalityGraph
  // container) vs the container/file stand-ins the provider appended.
  const graphNodes: GraphViewNode[] = [];
  for (const n of input.nodes) {
    if (n.id.startsWith("container:") || n.id.startsWith("file:")) continue;
    graphNodes.push(n);
  }

  // Degree (for sizing and for cap priority).
  const degreeById = new Map<string, number>();
  for (const e of input.edges) {
    degreeById.set(e.from, (degreeById.get(e.from) ?? 0) + 1);
    degreeById.set(e.to, (degreeById.get(e.to) ?? 0) + 1);
  }

  // Cap the drawn graph nodes honestly: keep the most-connected, never drop a
  // whole hub's presence (the hub itself is always drawn).
  let keptGraphNodes = graphNodes;
  if (graphNodes.length > MAX_GRAPH_NODES) {
    keptGraphNodes = [...graphNodes]
      .sort((a, b) => (degreeById.get(b.id) ?? 0) - (degreeById.get(a.id) ?? 0) || (a.id < b.id ? -1 : 1))
      .slice(0, MAX_GRAPH_NODES);
  }
  const keptSet = new Set(keptGraphNodes.map((n) => n.id));

  // Group the kept nodes by their source graph container.
  const perContainer = new Map<number, GraphViewNode[]>();
  for (const n of keptGraphNodes) {
    const list = perContainer.get(n.sourceContainerId) ?? [];
    list.push(n);
    perContainer.set(n.sourceContainerId, list);
  }

  // Hubs: every graph container (even one whose nodes were all capped out).
  const hubIds = Object.keys(graphContainers)
    .map(Number)
    .sort((a, b) => {
      const ma = graphContainers[a].modality;
      const mb = graphContainers[b].modality;
      return ma < mb ? -1 : ma > mb ? 1 : a - b;
    });
  const hubShell = 16 + 3.2 * Math.sqrt(hubIds.length);
  const hubExtent = new Map<number, number>(); // hub → cluster radius
  hubIds.forEach((cid, i) => {
    const meta = graphContainers[cid];
    const members = perContainer.get(cid) ?? [];
    const localR = Math.min(13, 2.4 + 0.5 * Math.sqrt(members.length));
    hubExtent.set(cid, localR);
    const [x, y, z] = fib(i, Math.max(hubIds.length, 2));
    const idx = push({
      id: `hub:${cid}`,
      kind: "hub",
      pos: [x * hubShell, y * hubShell * 0.75, z * hubShell],
      color: MODALITY_COLOR[meta.modality] ?? "#8b949e",
      label: meta.name ?? `${meta.modality} graph #${cid}`,
      nodeType: "ModalityGraph",
      modality: meta.modality,
      radius: 0.9,
      shape: "sphere",
      isReal: true,
      containerId: cid,
      shell: localR + 1.4,
    });
    hubByContainer.set(cid, idx);
    entityByContainer.set(cid, idx);
  });

  // Graph nodes: a cluster around their hub, grouped by node_type.
  const modalityCounts: Record<string, number> = {};
  for (const cid of hubIds) {
    const hubIdx = hubByContainer.get(cid)!;
    const hubPos = nodes[hubIdx].pos;
    const members = (perContainer.get(cid) ?? [])
      .slice()
      .sort((a, b) => (a.nodeType < b.nodeType ? -1 : a.nodeType > b.nodeType ? 1 : a.id < b.id ? -1 : 1));
    const localR = hubExtent.get(cid) ?? 3;
    members.forEach((n, i) => {
      const [fx, fy, fz] = fib(i, Math.max(members.length, 2));
      const jitter = 0.78 + 0.22 * ((hashStr(n.id) % 100) / 100);
      const deg = degreeById.get(n.id) ?? 0;
      push({
        id: n.id,
        kind: "node",
        pos: [hubPos[0] + fx * localR * jitter, hubPos[1] + fy * localR * jitter, hubPos[2] + fz * localR * jitter],
        color: MODALITY_COLOR[n.modality as Modality] ?? "#8b949e",
        label: clip(n.label, 40),
        nodeType: n.nodeType,
        modality: String(n.modality),
        radius: 0.3 + Math.min(deg * 0.05, 0.55),
        shape: shapeFor(String(n.modality)),
        isReal: n.isReal !== false,
        containerId: n.sourceContainerId,
      });
      modalityCounts[String(n.modality)] = (modalityCounts[String(n.modality)] ?? 0) + 1;
    });
  }
  // Graph nodes whose container is not a known hub (should not happen — the
  // provider only converts known-modality graphs) still get drawn, never lost.
  const orphanNodes = keptGraphNodes.filter((n) => !byId.has(n.id));
  orphanNodes.forEach((n, i) => {
    const [fx, fy, fz] = fib(i, Math.max(orphanNodes.length, 2));
    push({
      id: n.id,
      kind: "node",
      pos: [fx * hubShell * 0.5, fy * hubShell * 0.5, fz * hubShell * 0.5],
      color: MODALITY_COLOR[n.modality as Modality] ?? "#8b949e",
      label: clip(n.label, 40),
      nodeType: n.nodeType,
      modality: String(n.modality),
      radius: 0.35,
      shape: shapeFor(String(n.modality)),
      isReal: n.isReal !== false,
      containerId: n.sourceContainerId,
    });
  });

  // Files: an outer ring, each pulled toward the hubs it feeds.
  const sortedFiles = [...files].sort((a, b) => a.containerId - b.containerId);
  const shownFiles = sortedFiles.slice(0, MAX_FILE_NODES);
  const fileShell = hubShell * 1.45;
  shownFiles.forEach((f, i) => {
    const linked = f.linkedContainers
      .map((c) => hubByContainer.get(c))
      .filter((x): x is number => x !== undefined);
    let pos: [number, number, number];
    if (linked.length > 0) {
      const cx = linked.reduce((s, h) => s + nodes[h].pos[0], 0) / linked.length;
      const cy = linked.reduce((s, h) => s + nodes[h].pos[1], 0) / linked.length;
      const cz = linked.reduce((s, h) => s + nodes[h].pos[2], 0) / linked.length;
      const len = Math.max(1e-6, Math.hypot(cx, cy, cz));
      const k = Math.max(fileShell, len * 1.3) / len;
      // fan out files that share the same target so they do not stack
      const fan = (i % 5) - 2;
      pos = [cx * k + fan * 1.1, cy * k + (i % 3) * 0.8, cz * k - fan * 1.1];
    } else {
      const a = (i / Math.max(shownFiles.length, 1)) * Math.PI * 2;
      pos = [Math.cos(a) * fileShell, -hubShell * 0.95, Math.sin(a) * fileShell];
    }
    const idx = push({
      id: `file:${f.containerId}`,
      kind: "file",
      pos,
      color: C_FILE,
      label: clip(f.name, 40),
      nodeType: f.kind,
      modality: "container",
      radius: 0.85,
      shape: "box",
      isReal: true,
      containerId: f.containerId,
    });
    entityByContainer.set(f.containerId, idx);
  });

  // Other containers: an outer shell, grouped by container type, newest first.
  const allContainers = Object.values(containers).sort((a, b) =>
    a.containerType < b.containerType ? -1 : a.containerType > b.containerType ? 1 : b.containerId - a.containerId,
  );
  const shownContainers = allContainers.slice(0, MAX_CONTAINER_NODES);
  const shellC = hubShell * 1.9;
  shownContainers.forEach((c, i) => {
    const [fx, fy, fz] = fib(i, Math.max(shownContainers.length, 2));
    const idx = push({
      id: `container:${c.containerId}`,
      kind: "container",
      pos: [fx * shellC, fy * shellC * 0.7, fz * shellC],
      color: CONTAINER_PALETTE[hashStr(c.containerType) % CONTAINER_PALETTE.length],
      label: clip(c.name ?? `${c.containerType} #${c.containerId}`, 40),
      nodeType: c.containerType,
      modality: "container",
      radius: 0.85,
      shape: "octa",
      isReal: true,
      containerId: c.containerId,
    });
    entityByContainer.set(c.containerId, idx);
  });

  // ── Edges ──
  const edges: WorldEdge[] = [];
  const seenEdge = new Set<string>();
  const addEdge = (a: number | undefined, b: number | undefined, cls: WorldEdgeClass) => {
    if (a === undefined || b === undefined || a === b) return;
    const key = a < b ? `${a}|${b}|${cls}` : `${b}|${a}|${cls}`;
    if (seenEdge.has(key)) return;
    seenEdge.add(key);
    edges.push({ a, b, cls });
    nodes[a].degree++;
    nodes[b].degree++;
  };

  for (const e of input.edges) {
    if (!keptSet.has(e.from) || !keptSet.has(e.to)) continue;
    const cls: WorldEdgeClass =
      e.edgeClass === "structural" || e.edgeClass === "dependency" || e.edgeClass === "semantic" ||
      e.edgeClass === "cross-modal" || e.edgeClass === "lineage" || e.edgeClass === "governance"
        ? e.edgeClass
        : "semantic";
    addEdge(byId.get(e.from), byId.get(e.to), cls);
  }
  for (const cid of hubIds) addEdge(rootIdx, hubByContainer.get(cid), "root");
  for (const link of crossLinks) {
    const a = entityByContainer.get(link.sourceContainer);
    const b = entityByContainer.get(link.targetContainer);
    const srcMod = graphContainers[link.sourceContainer]?.modality;
    const dstMod = graphContainers[link.targetContainer]?.modality;
    const cross = !!srcMod && !!dstMod && srcMod !== dstMod && link.relationType === "SimilarTo";
    addEdge(a, b, cross ? "cross-modal" : "link");
  }
  for (const f of shownFiles) {
    const fi = byId.get(`file:${f.containerId}`);
    for (const c of f.linkedContainers) addEdge(fi, hubByContainer.get(c), "file");
  }

  let radius = 8;
  for (const n of nodes) radius = Math.max(radius, Math.hypot(n.pos[0], n.pos[1], n.pos[2]) + n.radius);

  return {
    nodes,
    edges,
    byId,
    hubByContainer,
    entityByContainer,
    radius,
    modalityCounts,
    truncation: {
      graphNodes: { shown: keptGraphNodes.length, total: graphNodes.length },
      containers: { shown: shownContainers.length, total: allContainers.length },
      files: { shown: shownFiles.length, total: sortedFiles.length },
    },
  };
}

// ─── Lens highlighting ───

export type WorldLens =
  | { kind: "graph" }
  | { kind: "files" }
  | { kind: "fabric" }
  | { kind: "hierarchy" }
  /** `modalities` = the modality names this engine tab covers (empty = all);
   * `label` = the engine tab's own name for the honest lens line. */
  | { kind: "engines"; modalities: string[]; label: string };

export interface LensView {
  /** null = everything active (no dimming). */
  active: Set<number> | null;
  /** Edge classes drawn emphasised for this lens. */
  emphasize: Set<WorldEdgeClass>;
  /** One honest line saying what the lens highlights. */
  label: string;
  /** World-space point + radius the camera frames for this lens. */
  focus: { center: [number, number, number]; radius: number };
}

const HIERARCHY_TYPE = /amt|derived|lineage|jurisdiction|blueprint|methodolog/i;

export function computeLensView(layout: WorldLayout, lens: WorldLens, crossLinks: GKCrossLink[], files: GKFileRef[]): LensView {
  const whole = { center: [0, 0, 0] as [number, number, number], radius: layout.radius };
  const frame = (idxs: Iterable<number>): LensView["focus"] => {
    const list = Array.from(idxs);
    if (list.length === 0) return whole;
    let cx = 0, cy = 0, cz = 0;
    for (const i of list) {
      cx += layout.nodes[i].pos[0];
      cy += layout.nodes[i].pos[1];
      cz += layout.nodes[i].pos[2];
    }
    cx /= list.length; cy /= list.length; cz /= list.length;
    let r = 6;
    for (const i of list) {
      const p = layout.nodes[i].pos;
      r = Math.max(r, Math.hypot(p[0] - cx, p[1] - cy, p[2] - cz) + layout.nodes[i].radius);
    }
    return { center: [cx, cy, cz], radius: r };
  };
  const nodesOfContainers = (cids: Set<number>): Set<number> => {
    const out = new Set<number>();
    for (const n of layout.nodes) {
      if (n.kind === "hub" || n.kind === "node") {
        if (n.containerId !== null && cids.has(n.containerId)) out.add(n.index);
      }
    }
    return out;
  };

  switch (lens.kind) {
    case "graph":
      return { active: null, emphasize: new Set(), label: "Graph — every container, node and link", focus: whole };

    case "files": {
      const act = new Set<number>();
      const fed = new Set<number>();
      for (const f of files) {
        const fi = layout.byId.get(`file:${f.containerId}`);
        if (fi !== undefined) act.add(fi);
        for (const c of f.linkedContainers) fed.add(c);
      }
      for (const i of nodesOfContainers(fed)) act.add(i);
      act.add(0);
      return {
        active: act,
        emphasize: new Set(["file"]),
        label: files.length
          ? `Files — ${files.length} linked file${files.length === 1 ? "" : "s"} and the graphs they feed`
          : "Files — no linked files in this project yet",
        focus: frame(act),
      };
    }

    case "fabric": {
      // The provider's cross-link index also carries store-wide pairs from
      // GetCrossModalPairs; only links that touch THIS project's world count.
      const inWorld = crossLinks.filter(
        (l) => layout.entityByContainer.has(l.sourceContainer) || layout.entityByContainer.has(l.targetContainer),
      );
      const linked = new Set<number>();
      for (const l of inWorld) {
        linked.add(l.sourceContainer);
        linked.add(l.targetContainer);
      }
      const act = nodesOfContainers(linked);
      for (const c of linked) {
        const idx = layout.entityByContainer.get(c);
        if (idx !== undefined) act.add(idx);
      }
      act.add(0);
      return {
        active: act,
        emphasize: new Set(["cross-modal", "link"]),
        label:
          inWorld.length > 0
            ? `Fabric — ${inWorld.length} container-level link${inWorld.length === 1 ? "" : "s"} touch this project (cross-modal in magenta)`
            : "Fabric — no container-level links touch this project's containers yet",
        focus: frame(act),
      };
    }

    case "hierarchy": {
      const act = new Set<number>();
      for (const n of layout.nodes) {
        if (n.kind === "container" && HIERARCHY_TYPE.test(n.nodeType)) act.add(n.index);
      }
      act.add(0);
      return {
        active: act,
        emphasize: new Set(["lineage", "governance"]),
        label: "Hierarchy — AMT, lineage, plan and jurisdiction containers",
        focus: frame(act),
      };
    }

    case "engines": {
      const want = new Set(lens.modalities);
      if (want.size === 0) {
        return {
          active: null,
          emphasize: new Set(["structural", "dependency", "semantic"]),
          label: `Engines — ${lens.label}: every modality`,
          focus: whole,
        };
      }
      const act = new Set<number>();
      let drawn = 0;
      for (const n of layout.nodes) {
        if ((n.kind === "hub" || n.kind === "node") && want.has(n.modality)) {
          act.add(n.index);
          if (n.kind === "node") drawn++;
        }
      }
      return {
        active: act,
        emphasize: new Set(["structural", "dependency", "semantic"]),
        label: `Engines — ${lens.label}: ${drawn} node${drawn === 1 ? "" : "s"} drawn${drawn === 0 ? " (no graph of this modality in the project yet)" : ""}`,
        focus: frame(act),
      };
    }
  }
}
