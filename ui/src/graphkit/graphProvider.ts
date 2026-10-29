/**
 * GraphKit — GraphProvider. THE single source of truth for a project's graph
 * substrate in the UI (docs/GRAPH_NATIVE_UIUX_GUIDE.md §4.1).
 *
 * One fetch per project (project container → ALL children partitioned into
 * modality graphs / file refs / other containers → graph contents → cross-
 * links from real context.relationships + the host's GetCrossModalPairs
 * rebuild), ONE WebSocket subscription (the shared graphEventClient
 * singleton — invalidate-and-refetch, unchanged semantics), selectors for
 * every lens. Views stop fetching: ten open views = one fetch.
 *
 * Built ON the existing bones, not beside them: conversion + discriminator
 * + content reads are graphViewData's own (now-exported) functions, and the
 * discriminator is the single-source all-27 table in graphRenderers/defaults.
 */
import React from "react";
import { create } from "zustand";
import { zseiQuery } from "../ozoneClient";
import { getGraphEventClient } from "../graphEventClient";
import {
  convertGraph,
  getContainer,
  getContainerContent,
  type ContainerJson,
} from "../graphViewData";
import { modalityFromObjectStorePath } from "../graphRenderers/defaults";
import type {
  ContainerRelation,
  GraphViewEdge,
  GraphViewNode,
  Modality,
} from "../graphViewTypes";
import type { GKContainerRef, GKCrossLink, GKFileRef, GKStatus } from "./types";

const FILE_KINDS = new Set(["FileReference", "URLReference", "PackageReference"]);

export interface GraphKitState {
  projectId: number | null;
  status: GKStatus;
  nodes: GraphViewNode[];
  edges: GraphViewEdge[];
  /** Per-graph-container relations exactly as fetched (the canvas overlays'
   * existing contract). */
  containerRelations: Record<number, ContainerRelation[]>;
  /** ModalityGraph containers whose prefix no known modality claims —
   * counted, never silently dropped. */
  unknownModalityGraphs: number;
  sourceContainers: number[];
  files: GKFileRef[];
  containers: Record<number, GKContainerRef>;
  /** Every ModalityGraph container under the project (known modality only),
   * keyed by container id — the "hub" the 3D world draws each graph around. */
  graphContainers: Record<number, { modality: Modality; name: string | null }>;
  /** The project container's own name (null when unnamed) — the root hub. */
  projectName: string | null;
  crossLinks: GKCrossLink[];
  crossLinksByContainer: Record<number, GKCrossLink[]>;
  /** Real node-construction events, newest timestamp per composite node id
   * (`${modality}:${graphId}:${nodeId}`) — what the 3D world pulses. Pruned
   * to the last 10 seconds; empty when nothing is being constructed. */
  pulses: Record<string, number>;
  /** Unix-ms of the last real WS event for this project (presence indicators
   * show recency of REAL activity; 0 = none this session). */
  lastEventAt: number;
  /** Monotonic count of real node-construction events received for this
   * project this session (the 3D pulse pattern's counter, substrate-wide). */
  nodeEventCount: number;
  load: (projectId: number) => void;
  reset: () => void;
}

function chunk<T>(items: T[], size: number): T[][] {
  const out: T[][] = [];
  for (let i = 0; i < items.length; i += size) out.push(items.slice(i, i + size));
  return out;
}

async function fetchProjectSubstrate(projectId: number, signal: { cancelled: boolean }) {
  const project = await getContainer(projectId);
  if (!project) throw new Error(`Project container ${projectId} not found`);
  const childIds = project.global_state.child_ids;

  const children: { id: number; c: ContainerJson }[] = [];
  for (const batch of chunk(childIds, 8)) {
    const res = await Promise.all(batch.map((id) => getContainer(id)));
    batch.forEach((id, j) => {
      if (res[j]) children.push({ id, c: res[j]! });
    });
    if (signal.cancelled) return null;
  }

  const nodes: GraphViewNode[] = [];
  const edges: GraphViewEdge[] = [];
  const sourceContainers: number[] = [];
  const containerRelations: Record<number, ContainerRelation[]> = {};
  const files: GKFileRef[] = [];
  const containers: Record<number, GKContainerRef> = {};
  const graphContainers: Record<number, { modality: Modality; name: string | null }> = {};
  let unknownModalityGraphs = 0;
  const labels = new Map<number, string>();
  const modalities = new Map<number, Modality | null>();

  for (const { id, c } of children) {
    const type = c.local_state.metadata.container_type;
    const name = c.local_state.metadata.name ?? null;
    labels.set(id, name ?? `${type} #${id}`);
    if (type === "ModalityGraph") {
      const path = c.local_state.storage.object_store_path ?? null;
      // Single-source discriminator (graphRenderers/defaults) — all 27
      // modalities, auto-extending; null = counted, not dropped.
      const modality = modalityFromObjectStorePath(path);
      modalities.set(id, modality);
      containerRelations[id] = c.local_state.context.relationships ?? [];
      if (modality === null) {
        unknownModalityGraphs += 1;
        continue;
      }
      const raw = await getContainerContent(id);
      if (!raw) continue; // honest: container exists, content not JSON
      sourceContainers.push(id);
      graphContainers[id] = { modality, name };
      const conv = convertGraph(raw, modality, id);
      nodes.push(...conv.nodes);
      edges.push(...conv.edges);
      continue;
    }
    if (FILE_KINDS.has(type)) {
      const locator =
        (c.local_state.storage.object_store_path as string | null | undefined) ?? null;
      files.push({
        containerId: id,
        kind: type as GKFileRef["kind"],
        name: name ?? locator ?? `${type} #${id}`,
        locator,
        linkedContainers: (c.local_state.context.relationships ?? [])
          .map((r) => r.target_id)
          .filter((t) => t !== id),
      });
      continue;
    }
    containers[id] = {
      containerId: id,
      containerType: type,
      name,
      keywords: c.local_state.context.keywords ?? [],
      sourceFile: null,
    };
  }

  // REVERSE FILE WEAVE (wave 8): containers learn which file they came from —
  // the mirror of each file's own linkedContainers, so a container node can
  // answer "derived from file X" without the no-dead-ends law breaking at the
  // container side. Real relationship data, first writer wins.
  for (const f of files) {
    for (const cid of f.linkedContainers) {
      const target = containers[cid];
      if (target && target.sourceFile === null) {
        target.sourceFile = { containerId: f.containerId, name: f.name };
      }
    }
  }

  // Cross-links: real context.relationships of every fetched container + the
  // host's per-request pairs rebuild. Both sources are the store's own truth;
  // exact duplicates deduped, symmetric pairs kept as stored.
  const crossLinks: GKCrossLink[] = [];
  const seen = new Set<string>();
  const pushLink = (
    src: number,
    rel: { target_id: number; relation_type: string; confidence?: number; discovered_via?: string },
  ) => {
    if (src === rel.target_id) return;
    const key = `${src}>${rel.target_id}>${rel.relation_type}`;
    if (seen.has(key)) return;
    seen.add(key);
    crossLinks.push({
      sourceContainer: src,
      targetContainer: rel.target_id,
      relationType: rel.relation_type,
      confidence: rel.confidence ?? 1.0,
      discoveredVia: rel.discovered_via ?? "unknown",
      targetLabel: labels.get(rel.target_id) ?? null,
      targetModality: modalities.get(rel.target_id) ?? null,
    });
  };
  for (const [idKey, rels] of Object.entries(containerRelations)) {
    const src = Number(idKey);
    for (const rel of rels) pushLink(src, rel);
  }
  try {
    const resp = await zseiQuery<any>({ GetCrossModalPairs: { src: null, dst: null } });
    const content = resp && typeof resp === "object" && "Content" in resp ? (resp as any).Content : resp;
    const pairs = content?.json?.pairs ?? content?.pairs;
    if (Array.isArray(pairs)) {
      for (const p of pairs) {
        if (typeof p.source_id === "number" && typeof p.target_id === "number") {
          pushLink(p.source_id, {
            target_id: p.target_id,
            relation_type: typeof p.relation_type === "string" ? p.relation_type : "SimilarTo",
            confidence: typeof p.confidence === "number" ? p.confidence : 1.0,
            discovered_via: typeof p.discovered_via === "string" ? p.discovered_via : "CrossModalIndex",
          });
        }
      }
    }
  } catch {
    /* pairs read-model unavailable — relationship-derived links stand alone */
  }

  const crossLinksByContainer: Record<number, GKCrossLink[]> = {};
  for (const link of crossLinks) {
    (crossLinksByContainer[link.sourceContainer] ??= []).push(link);
    // Reverse index so an entity shows links pointed AT it too (both
    // directions of the fabric, the no-dead-ends law).
    (crossLinksByContainer[link.targetContainer] ??= []).push({
      ...link,
      sourceContainer: link.targetContainer,
      targetContainer: link.sourceContainer,
      targetLabel: labels.get(link.sourceContainer) ?? null,
      targetModality: modalities.get(link.sourceContainer) ?? null,
    });
  }

  // Convert other containers + files into renderable 3D nodes — without
  // this, text-only projects (no ModalityGraphs) render as empty 3D space.
  const containerNodes: GraphViewNode[] = [];
  for (const [cid, cref] of Object.entries(containers)) {
    const id = Number(cid);
    containerNodes.push({
      id: `container:${id}`,
      modality: "container",
      nodeType: cref.containerType,
      label: cref.name ?? `${cref.containerType} #${id}`,
      isReal: true,
      sourceContainerId: id,
      raw: { node_id: id, node_type: cref.containerType, properties: {} },
    });
  }
  const fileNodes: GraphViewNode[] = files.map(f => ({
    id: `file:${f.containerId}`,
    modality: "container" as const,
    nodeType: f.kind,
    label: f.name,
    isReal: true,
    sourceContainerId: f.containerId,
    raw: { node_id: f.containerId, node_type: f.kind, properties: {} },
  }));

  return {
    nodes: [...nodes, ...containerNodes, ...fileNodes],
    edges,
    sourceContainers,
    containerRelations,
    unknownModalityGraphs,
    files,
    containers,
    graphContainers,
    projectName: project.local_state.metadata.name ?? null,
    crossLinks,
    crossLinksByContainer,
  };
}

// Live-subscription bookkeeping (module-level, ONE substrate at a time). The
// first version registered handlers per load() and never released them, and
// every refetch closed over the FIRST load's cancellation flag — so switching
// projects left the old project's handlers refetching into the new one's
// state. Now: load()/reset() cancel the in-flight signal and release every
// handler before wiring the next project.
let activeSignal: { cancelled: boolean } | null = null;
let releaseHandlers: Array<() => void> = [];
let pendingPulses: Record<string, number> = {};
let pulseTimer: ReturnType<typeof setTimeout> | null = null;

function teardownLive() {
  if (activeSignal) activeSignal.cancelled = true;
  activeSignal = null;
  for (const release of releaseHandlers) {
    try {
      release();
    } catch {
      /* already released */
    }
  }
  releaseHandlers = [];
  pendingPulses = {};
  if (pulseTimer) {
    clearTimeout(pulseTimer);
    pulseTimer = null;
  }
}

const EMPTY_SUBSTRATE = {
  nodes: [] as GraphViewNode[],
  edges: [] as GraphViewEdge[],
  containerRelations: {} as Record<number, ContainerRelation[]>,
  unknownModalityGraphs: 0,
  sourceContainers: [] as number[],
  files: [] as GKFileRef[],
  containers: {} as Record<number, GKContainerRef>,
  graphContainers: {} as Record<number, { modality: Modality; name: string | null }>,
  projectName: null as string | null,
  crossLinks: [] as GKCrossLink[],
  crossLinksByContainer: {} as Record<number, GKCrossLink[]>,
};

type Substrate = NonNullable<Awaited<ReturnType<typeof fetchProjectSubstrate>>>;

function applySubstrate(result: Substrate, emptyWhy: string) {
  const empty =
    result.nodes.length === 0 &&
    result.files.length === 0 &&
    Object.keys(result.containers).length === 0;
  return {
    status: (empty ? { kind: "empty", why: emptyWhy } : { kind: "ready" }) as GKStatus,
    nodes: result.nodes,
    edges: result.edges,
    containerRelations: result.containerRelations,
    unknownModalityGraphs: result.unknownModalityGraphs,
    sourceContainers: result.sourceContainers,
    files: result.files,
    containers: result.containers,
    graphContainers: result.graphContainers,
    projectName: result.projectName,
    crossLinks: result.crossLinks,
    crossLinksByContainer: result.crossLinksByContainer,
  };
}

const PULSE_WINDOW_MS = 10_000;

export const useGraphKit = create<GraphKitState>((set, get) => ({
  projectId: null,
  status: { kind: "idle" },
  ...EMPTY_SUBSTRATE,
  pulses: {},
  lastEventAt: 0,
  nodeEventCount: 0,
  reset: () => {
    teardownLive();
    set({
      projectId: null,
      status: { kind: "idle" },
      ...EMPTY_SUBSTRATE,
      pulses: {},
      lastEventAt: 0,
      nodeEventCount: 0,
    });
  },
  load: (projectId: number) => {
    if (get().projectId === projectId && get().status.kind !== "idle") return;
    // Switching projects: drop the previous project's handlers/in-flight
    // fetches before anything else, then clear its substrate so the new
    // project never renders the old one's nodes while loading.
    teardownLive();
    set({
      projectId,
      status: { kind: "loading" },
      ...EMPTY_SUBSTRATE,
      pulses: {},
      lastEventAt: 0,
      nodeEventCount: 0,
    });
    const signal = { cancelled: false };
    activeSignal = signal;

    fetchProjectSubstrate(projectId, signal)
      .then((result) => {
        if (signal.cancelled || result === null) return;
        set(
          applySubstrate(
            result,
            "This project has no graph containers yet — run an orchestration with attachments, or link files, and this workspace fills from the graph.",
          ),
        );
      })
      .catch((err: unknown) => {
        if (signal.cancelled) return;
        set({
          status: {
            kind: "error",
            message: err instanceof Error ? err.message : String(err),
          },
        });
      });

    // ONE subscription per store (the shared client; never per-view sockets).
    // Graph events for this project's scope → honest refetch of the substrate.
    // Refetch is coalesced: a burst of events (a construction run emits many)
    // triggers one trailing refetch, never a refetch per event, and the
    // current substrate stays on screen (status stays "ready") meanwhile.
    const client = getGraphEventClient();
    client.connect();
    let refetchTimer: ReturnType<typeof setTimeout> | null = null;
    let refetching = false;
    let again = false;
    const refetch = () => {
      if (signal.cancelled) return;
      if (refetching) {
        again = true;
        return;
      }
      refetching = true;
      fetchProjectSubstrate(projectId, signal)
        .then((result) => {
          if (signal.cancelled || result === null) return;
          set(applySubstrate(result, "No graph containers under this project."));
        })
        .catch(() => {
          /* keep the previous substrate; the next event retries */
        })
        .finally(() => {
          refetching = false;
          if (again && !signal.cancelled) {
            again = false;
            refetch();
          }
        });
    };
    const scheduleRefetch = () => {
      if (refetchTimer) clearTimeout(refetchTimer);
      refetchTimer = setTimeout(refetch, 600);
    };
    releaseHandlers.push(() => {
      if (refetchTimer) clearTimeout(refetchTimer);
    });
    releaseHandlers.push(
      client.onEvent(
        (frame) => {
          if (frame.parent_id !== projectId && frame.container_id !== projectId) return;
          set({ lastEventAt: Date.now() });
          scheduleRefetch();
        },
        [`proj:${projectId}`, "scope:global"],
      ),
    );
    // Node-construction events → substrate-wide pulse counter AND per-node
    // pulses (the 3D pattern promoted: the world pulses exactly the node the
    // modality pipeline just constructed). Batched to <=4 store updates/sec —
    // producers can fire hundreds of events per graph.
    releaseHandlers.push(
      client.onNodeEvent((frame) => {
        if (frame.project_id !== projectId) return;
        const now = Date.now();
        pendingPulses[`${frame.modality}:${frame.graph_id}:${frame.node_id}`] = now;
        if (frame.related_node_id != null) {
          pendingPulses[`${frame.modality}:${frame.graph_id}:${frame.related_node_id}`] = now;
        }
        set((s) => ({ nodeEventCount: s.nodeEventCount + 1, lastEventAt: now }));
        if (pulseTimer) return;
        pulseTimer = setTimeout(() => {
          pulseTimer = null;
          const flush = pendingPulses;
          pendingPulses = {};
          const cutoff = Date.now() - PULSE_WINDOW_MS;
          set((s) => {
            const merged: Record<string, number> = {};
            for (const [k, t] of Object.entries(s.pulses)) if (t >= cutoff) merged[k] = t;
            for (const [k, t] of Object.entries(flush)) merged[k] = t;
            return { pulses: merged };
          });
        }, 250);
      }),
    );
  },
}));

/** Hook for views: point at a project once; the substrate loads and stays
 * live. Re-renders only on state the selector reads. Effects (not render)
 * drive load/reset — a zustand set during render is a side effect. */
export function useGraphKitProject(projectId: number | null) {
  const load = useGraphKit((s) => s.load);
  const reset = useGraphKit((s) => s.reset);
  const current = useGraphKit((s) => s.projectId);
  React.useEffect(() => {
    if (projectId === null) {
      if (current !== null) reset();
      return;
    }
    load(projectId);
  }, [projectId, current, load, reset]);
}
