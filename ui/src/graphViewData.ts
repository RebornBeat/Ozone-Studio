/**
 * Graph View data-fetching (Batch C, fork C1) — the real B1 fetch sequence
 * (confirmed already servable, no new backend work) plus live invalidation
 * over the real B19 WebSocket client.
 *
 * Fetch sequence per docs/UI_UX_FORK_PLAN.md's B1 entry:
 *   1. GetContainer{project_id}                → global_state.child_ids
 *   2. GetContainer{child_id} per child         → keep container_type=="ModalityGraph"
 *   3. GetContainerContent{that_id}             → real {nodes, edges, ...} JSON
 *
 * Real per-modality disambiguation (found while building this fork, not in
 * the original plan): `local_state.metadata.modality` is NOT a reliable
 * discriminant — code/text pipelines correctly set it to the real enum
 * values "Code"/"Text", but math sets it to "Structured" because the
 * shared `Modality` enum (src/types/container.rs) has no "Math" variant at
 * all (documented in math's own persist_graph_container). The reliable
 * discriminant used here instead is `local_state.storage.object_store_path`,
 * which is always exactly `graphs/{code|math|text}_{id}.json` — unambiguous,
 * and already present on the GetContainer response with no extra call.
 */

import { zseiQuery } from "./ozoneClient";
import { GraphEventFrame, getGraphEventClient } from "./graphEventClient";
import { classifyEdge, classifyNode } from "./graphRenderers";
import {
  ContainerRelation,
  GraphViewData,
  GraphViewEdge,
  GraphViewNode,
  GraphViewStatus,
  Modality,
  RawGraphEdge,
  RawGraphNode,
  RawModalityGraph,
} from "./graphViewTypes";

// ── Real ZSEIQueryResult parsing (externally-tagged JSON: {"Variant": {...}}) ──

interface ContainerJson {
  global_state: { container_id: number; parent_id: number; child_ids: number[] };
  local_state: {
    metadata: { container_type: string; modality: string; name?: string | null };
    context: { relationships: ContainerRelation[] };
    storage: { object_store_path?: string | null };
  };
}

async function getContainer(containerId: number): Promise<ContainerJson | null> {
  const result = await zseiQuery<any>({ GetContainer: { container_id: containerId } });
  if (result && typeof result === "object" && "Container" in result) {
    return result.Container as ContainerJson;
  }
  return null;
}

async function getContainerContent(containerId: number): Promise<RawModalityGraph | null> {
  const result = await zseiQuery<any>({ GetContainerContent: { container_id: containerId } });
  const content = result && typeof result === "object" ? result.Content : null;
  if (!content) return null;
  // Honest per B0's contract: json is null when the file wasn't JSON, or
  // when the container has no object_store_path at all. Never synthesize
  // graph data from `raw` — a non-JSON content file isn't a graph.
  return (content.json as RawModalityGraph) ?? null;
}

function modalityFromObjectStorePath(path: string | null | undefined): Modality | null {
  if (!path) return null;
  if (path.startsWith("graphs/code_")) return "code";
  if (path.startsWith("graphs/math_")) return "math";
  if (path.startsWith("graphs/text_")) return "text";
  if (path.startsWith("graphs/image_")) return "image";
  return null;
}

function nodeLabel(node: RawGraphNode): string {
  // Real field presence genuinely differs per modality (see graphViewTypes.ts) —
  // prefer whichever real field is present rather than guessing one.
  return node.label ?? node.name ?? (node.content ? node.content.slice(0, 60) : `#${node.node_id}`);
}

function edgeId(edge: RawGraphEdge): string {
  return edge.edge_id !== undefined
    ? `e${edge.edge_id}`
    : `${edge.from_node}-${edge.edge_type}-${edge.to_node}`;
}

/** Structural conversion; the per-modality "is this type actually constructed
 * anywhere" judgment (isReal, edgeClass, label) is delegated to the renderer
 * plugins in ./graphRenderers (C2-C7). */
function convertGraph(
  raw: RawModalityGraph,
  modality: Modality,
  sourceContainerId: number,
): { nodes: GraphViewNode[]; edges: GraphViewEdge[] } {
  const nodes: GraphViewNode[] = raw.nodes.map((n) => {
    const c = classifyNode(modality, n);
    return {
      id: `${modality}:${sourceContainerId}:${n.node_id}`,
      modality,
      nodeType: n.node_type,
      label: c.label ?? nodeLabel(n),
      contentPreview: c.contentPreview ?? n.content,
      confidence: n.confidence,
      isReal: c.isReal,
      raw: n,
      sourceContainerId,
    };
  });

  const edges: GraphViewEdge[] = raw.edges.map((e) => {
    const c = classifyEdge(modality, e);
    return {
      id: `${modality}:${sourceContainerId}:${edgeId(e)}`,
      from: `${modality}:${sourceContainerId}:${e.from_node}`,
      to: `${modality}:${sourceContainerId}:${e.to_node}`,
      edgeType: e.edge_type,
      edgeClass: c.edgeClass,
      isReal: c.isReal,
      raw: e,
    };
  });

  return { nodes, edges };
}

export interface UseGraphDataOptions {
  /** Restrict to one modality's graph, or fetch all 3 found under the
   * project (default). */
  modality?: Modality | "all";
}

/**
 * Fetch every ModalityGraph under `projectId` (optionally filtered to one
 * modality), convert to the unified canvas contract, and stay live-updated
 * via the real B19 WebSocket client (invalidate-and-refetch on any
 * graph_event scoped to this project — confirmed the accepted design,
 * B18's writeup: frames are bare change-notifications, not diffs).
 *
 * Framework-agnostic on purpose (plain function + callback, not a React
 * hook) so it's usable from GraphView.tsx via a thin useEffect wrapper
 * without coupling this fetch/subscribe logic to React itself.
 */
export function loadGraphData(
  projectId: number,
  onStatus: (status: GraphViewStatus) => void,
  options: UseGraphDataOptions = {},
): () => void {
  const modalityFilter = options.modality ?? "all";
  let cancelled = false;

  async function fetchOnce() {
    onStatus({ kind: "loading" });
    try {
      const project = await getContainer(projectId);
      if (!project) {
        if (!cancelled) onStatus({ kind: "error", message: `Project container ${projectId} not found` });
        return;
      }

      const childIds = project.global_state.child_ids;
      const children = await Promise.all(childIds.map((id) => getContainer(id)));

      const modalityGraphChildren = children
        .map((c, i) => ({ container: c, id: childIds[i] }))
        .filter(
          (c): c is { container: ContainerJson; id: number } =>
            !!c.container && c.container.local_state.metadata.container_type === "ModalityGraph",
        )
        .map((c) => ({
          ...c,
          modality: modalityFromObjectStorePath(c.container.local_state.storage.object_store_path),
        }))
        .filter((c) => c.modality !== null && (modalityFilter === "all" || c.modality === modalityFilter));

      if (modalityGraphChildren.length === 0) {
        if (!cancelled) {
          onStatus({ kind: "empty" });
        }
        return;
      }

      const allNodes: GraphViewNode[] = [];
      const allEdges: GraphViewEdge[] = [];
      const sourceContainers: number[] = [];
      const containerRelations: Record<number, ContainerRelation[]> = {};

      for (const child of modalityGraphChildren) {
        const raw = await getContainerContent(child.id);
        sourceContainers.push(child.id);
        containerRelations[child.id] = child.container.local_state.context.relationships ?? [];
        if (!raw) continue; // honest: container exists, content file didn't resolve to JSON
        const { nodes, edges } = convertGraph(raw, child.modality as Modality, child.id);
        allNodes.push(...nodes);
        allEdges.push(...edges);
      }

      if (!cancelled) {
        onStatus({
          kind: "ready",
          data: { nodes: allNodes, edges: allEdges, sourceContainers, containerRelations },
        });
      }
    } catch (err) {
      if (!cancelled) {
        onStatus({ kind: "error", message: err instanceof Error ? err.message : String(err) });
      }
    }
  }

  fetchOnce();

  // Live invalidation: any graph_event for this project's scope triggers a
  // real re-fetch. Uses the process-wide singleton client (getGraphEventClient)
  // rather than opening a dedicated socket per graph view — the client does
  // not auto-connect, so connect() here is required; it's a no-op if some
  // other consumer already opened it. Only unsubscribe on cleanup, never
  // close() the shared client — that would drop every other subscriber.
  // Client-side scope filtering matches B19's contract — "proj:<id>" is the
  // real keyword shape confirmed in context_mirror.rs for project-scoped
  // events; global-scope events also always match.
  const wsClient = getGraphEventClient();
  wsClient.connect();
  const unsubscribe = wsClient.onEvent((frame: GraphEventFrame) => {
    if (frame.parent_id === projectId || frame.container_id === projectId) {
      fetchOnce();
    }
  }, [`proj:${projectId}`, "scope:global"]);

  return () => {
    cancelled = true;
    unsubscribe();
  };
}
