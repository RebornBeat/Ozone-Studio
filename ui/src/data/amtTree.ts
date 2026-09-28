/**
 * AMT tree loader (D3, per B6).
 *
 * An AMT generation's tree lives behind its `Derived` container's
 * `object_store_path` (e.g. `amt/amt_p3_1789903054_2.json`). `GetContainerContent`
 * (B0) returns `Content{json}`, and the JSON top level IS the root node itself
 * (no wrapper) — confirmed against the live host (container 30411) and the host's
 * data dir `target/release/zsei_data/amt/` (61 files, 27 with branches, max depth 3).
 *
 * Confirmed real shape (src/orchestrator/mod.rs:614 `AMTNode`, verified against disk):
 *  - `children` NEST FULL NODES (`Vec<AMTNode>`), not ids.
 *  - `metadata` is `HashMap<String,String>` — string values only (keys seen on disk:
 *    `source_sentence_<n>`, `type`).
 *  - `node_type` ∈ Root | Branch | Leaf | Consideration | CrossReference (CrossReference is
 *    declared but never appears on disk).
 *  - `verified` = "backed by source chunk evidence"; `confidence` is a derived 1.0/0.0 mirror of
 *    `verified`, NOT an independent score (struct doc comment) — do not present it as one.
 *  - `relationships[].relation_type` ∈ DependsOn | Requires | RelatesTo | Contradicts |
 *    Elaborates | SharedContext | Continues. Only `Continues` occurs on disk, and its `target_id`
 *    is the prior generation's AMT CONTAINER id, not a node id in this tree.
 *
 * NOTE on the query envelope: `zseiQuery()` resolves to the raw host envelope
 * `{success, result, error}` on both the Electron and browser paths, so the ZSEIQueryResult
 * variant (`{"Content": {...}}`) sits under `.result`. `unwrapZseiResult` handles that.
 */
import { zseiQuery } from "../ozoneClient";

export interface AmtRelationRaw {
  /** For `Continues`: the prior generation's AMT container id. Otherwise: a node id in the same tree. */
  target_id: number;
  relation_type: string;
  confidence: number;
}

export interface AmtNodeRaw {
  id: number;
  node_type: string;
  content: string;
  /** Indices of the source chunks that back this node (empty ⇒ unverified). */
  source_chunk_indices: number[];
  /** Nested full child nodes (confirmed against real files). */
  children: AmtNodeRaw[];
  relationships: AmtRelationRaw[];
  methodology_ids: number[];
  /** String values only (Rust `HashMap<String,String>`). */
  metadata: Record<string, string>;
  depth: number;
  verified: boolean;
  /** Derived 1.0/0.0 mirror of `verified` — not an independent confidence score. */
  confidence: number;
  [extra: string]: unknown;
}

/** Declared `AMTNodeType` variants (CrossReference has no real occurrence on disk). */
export const AMT_NODE_TYPES = ["Root", "Branch", "Leaf", "Consideration", "CrossReference"] as const;

/** Declared `AMTRelationType` variants (only `Continues` has real occurrences on disk). */
export const AMT_RELATION_TYPES = [
  "DependsOn",
  "Requires",
  "RelatesTo",
  "Contradicts",
  "Elaborates",
  "SharedContext",
  "Continues",
] as const;

/** Unwrap the host's `{success, result, error}` envelope; tolerate an already-unwrapped value. */
export function unwrapZseiResult<T = unknown>(resp: unknown): T {
  if (resp && typeof resp === "object" && "success" in resp && "result" in resp) {
    const r = resp as { success: boolean; result: unknown; error?: string | null };
    if (!r.success) throw new Error(r.error ?? "ZSEI query failed");
    return r.result as T;
  }
  return resp as T;
}

function isAmtNode(v: unknown): v is AmtNodeRaw {
  if (!v || typeof v !== "object") return false;
  const o = v as Record<string, unknown>;
  return (
    typeof o.id === "number" &&
    typeof o.node_type === "string" &&
    typeof o.content === "string" &&
    Array.isArray(o.children) &&
    Array.isArray(o.relationships)
  );
}

/**
 * Load one AMT generation's tree.
 * - resolves `null` when the container has no content file, or the file isn't JSON (honest absence);
 * - throws on a failed query or when the JSON isn't an AMT root node (never guesses a shape).
 */
export async function loadAmtTree(amtContainerId: number): Promise<AmtNodeRaw | null> {
  const resp = await zseiQuery<unknown>({ GetContainerContent: { container_id: amtContainerId } });
  const result = unwrapZseiResult<{ Content?: { json?: unknown; raw?: string | null } }>(resp);
  const content = result?.Content;
  if (!content) throw new Error(`GetContainerContent(${amtContainerId}): unexpected response shape`);
  if (content.json === null || content.json === undefined) return null;
  if (!isAmtNode(content.json)) {
    throw new Error(`Container ${amtContainerId}: content JSON is not an AMT root node`);
  }
  return content.json;
}

/** Depth-first flatten, parents before children. `path` is a stable key (child index path). */
export interface FlatAmtNode {
  node: AmtNodeRaw;
  parent: AmtNodeRaw | null;
  level: number;
  path: string;
}
export function flattenAmtTree(root: AmtNodeRaw): FlatAmtNode[] {
  const out: FlatAmtNode[] = [];
  const visit = (node: AmtNodeRaw, parent: AmtNodeRaw | null, level: number, path: string) => {
    out.push({ node, parent, level, path });
    node.children.forEach((c, i) => visit(c, node, level + 1, `${path}/${i}`));
  };
  visit(root, null, 0, "0");
  return out;
}
