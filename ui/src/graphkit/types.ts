/**
 * GraphKit — types. The substrate model: everything the UI shows is a
 * projection of the one ZSEI graph, so the kit's types name the projections,
 * not per-view datasets (docs/GRAPH_NATIVE_UIUX_GUIDE.md §4.1).
 *
 * Node/edge models REUSE the existing unified graph model (graphViewTypes)
 * verbatim — one conversion path (graphViewData.convertGraph), one composite
 * id format (`${modality}:${containerId}:${nodeId}`), one isReal discipline.
 * GraphKit adds what the graph-first substrate needs beyond a single canvas:
 * container refs, file refs, cross-links, and task projections.
 */
import type { GraphViewEdge, GraphViewNode, Modality } from "../graphViewTypes";

export type { GraphViewEdge, GraphViewNode, Modality };

/** Which projection an entity was reached through — a lens highlights its
 * own, but every entity remains reachable from every other (no silos). */
export type LensId =
  | "overview"
  | "graph"
  | "fabric"
  | "files"
  | "hierarchy"
  | "context"
  | "tasks"
  | "capture"
  | "coordination"
  | "network"
  | Modality; // engine lenses are per-modality

/** A first-class file-like container (graph-native FileReference family). */
export interface GKFileRef {
  containerId: number;
  kind: "FileReference" | "URLReference" | "PackageReference";
  name: string;
  /** Real object_store_path / url / package spec when present — never guessed. */
  locator: string | null;
  /** Container ids of the graphs this file is connected to (real
   * context.relationships of the file container). */
  linkedContainers: number[];
}

/** A non-graph, non-file container (methodology, blueprint, task record,
 * coordination event, …) — present in the substrate so cross-links resolve. */
export interface GKContainerRef {
  containerId: number;
  containerType: string;
  name: string | null;
  keywords: string[];
  /** REVERSE FILE WEAVE (wave 8): the FileReference this container was
   * derived from — the mirror of the file's own linkedContainers. Null for
   * containers with no file origin. Real relationship data, first writer
   * wins when several files reference one container. */
  sourceFile: { containerId: number; name: string } | null;
}

/** One cross-container link, resolved to citable labels. Sources: the real
 * `context.relationships` of every fetched container + the host's
 * per-request `GetCrossModalPairs` rebuild (never a second writer). */
export interface GKCrossLink {
  sourceContainer: number;
  targetContainer: number;
  relationType: string;
  confidence: number;
  discoveredVia: string;
  /** Resolved lazily by the provider from the containers it already holds —
   * null when the target container is not under this project (still real,
   * still navigable by container id). */
  targetLabel: string | null;
  targetModality: Modality | null;
}

/** Task projection for indicators — fed from the REAL task record
 * (services/store), never synthesized. Steps are the record's own statuses. */
export interface GKTaskRef {
  taskId: number;
  status: string;
  progress: number | null; // the record's own number, or null — never invented
  stepStatuses: string[];
  error: string | null;
}

/** Fetch status — the HonestState contract, codified. `empty` carries the
 * WHY (and, where known, what would make it non-empty). */
export type GKStatus =
  | { kind: "idle" }
  | { kind: "loading" }
  | { kind: "ready" }
  | { kind: "empty"; why: string }
  | { kind: "error"; message: string };
