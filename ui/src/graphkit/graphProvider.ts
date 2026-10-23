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

interface GraphKitState {
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
  crossLinks: GKCrossLink[];
  crossLinksByContainer: Record<number, GKCrossLink[]>;
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
    };
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

  return {
    nodes,
    edges,
    sourceContainers,
    containerRelations,
    unknownModalityGraphs,
    files,
    containers,
    crossLinks,
    crossLinksByContainer,
  };
}

export const useGraphKit = create<GraphKitState>((set, get) => ({
  projectId: null,
  status: { kind: "idle" },
  nodes: [],
  edges: [],
  containerRelations: {},
  unknownModalityGraphs: 0,
  sourceContainers: [],
  files: [],
  containers: {},
  crossLinks: [],
  crossLinksByContainer: {},
  lastEventAt: 0,
  nodeEventCount: 0,
  reset: () =>
    set({
      projectId: null,
      status: { kind: "idle" },
      nodes: [],
      edges: [],
      containerRelations: {},
      unknownModalityGraphs: 0,
      sourceContainers: [],
      files: [],
      containers: {},
      crossLinks: [],
      crossLinksByContainer: {},
      lastEventAt: 0,
      nodeEventCount: 0,
    }),
  load: (projectId: number) => {
    if (get().projectId === projectId && get().status.kind !== "idle") return;
    set({ projectId, status: { kind: "loading" } });
    const signal = { cancelled: false };

    fetchProjectSubstrate(projectId, signal)
      .then((result) => {
        if (signal.cancelled || result === null) return;
        const empty =
          result.nodes.length === 0 &&
          result.files.length === 0 &&
          Object.keys(result.containers).length === 0;
        set({
          status: empty
            ? {
                kind: "empty",
                why: "This project has no graph containers yet — run an orchestration with attachments, or link files, and this workspace fills from the graph.",
              }
            : { kind: "ready" },
          nodes: result.nodes,
          edges: result.edges,
          containerRelations: result.containerRelations,
          unknownModalityGraphs: result.unknownModalityGraphs,
          sourceContainers: result.sourceContainers,
          files: result.files,
          containers: result.containers,
          crossLinks: result.crossLinks,
          crossLinksByContainer: result.crossLinksByContainer,
        });
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
    const client = getGraphEventClient();
    client.connect();
    client.onEvent(
      (frame) => {
        if (frame.parent_id !== projectId && frame.container_id !== projectId) return;
        set({ lastEventAt: Date.now() });
        // Refetch by forcing the load path (projectId already set → reset
        // gate bypassed via explicit status bump).
        set({ status: { kind: "loading" } });
        fetchProjectSubstrate(projectId, signal)
          .then((result) => {
            if (signal.cancelled || result === null) return;
            set({
              status: result.nodes.length + result.files.length === 0 ? { kind: "empty", why: "No graph containers under this project." } : { kind: "ready" },
              nodes: result.nodes,
              edges: result.edges,
              containerRelations: result.containerRelations,
              unknownModalityGraphs: result.unknownModalityGraphs,
              sourceContainers: result.sourceContainers,
              files: result.files,
              containers: result.containers,
              crossLinks: result.crossLinks,
              crossLinksByContainer: result.crossLinksByContainer,
            });
          })
          .catch(() => {
            /* keep the previous substrate; next event retries */
            set({ status: { kind: "ready" } });
          });
      },
      [`proj:${projectId}`, "scope:global"],
    );
    // Node-construction events → substrate-wide pulse counter (the 3D
    // pattern promoted; per-node highlight stays a lens concern).
    client.onNodeEvent((frame) => {
      if (frame.project_id !== projectId) return;
      set((s) => ({ nodeEventCount: s.nodeEventCount + 1, lastEventAt: Date.now() }));
    });
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
