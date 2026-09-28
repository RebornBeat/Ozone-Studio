/**
 * AMT generation + fork/main lineage loader (D4; consumed by C9, D3, H4, J1).
 *
 * Real sources (docs/UI_UX_FORK_PLAN.md B6/B7/B8, confirmed against src/orchestrator/amt.rs:150-330 and the live host):
 *  - Every AMT generation, main and fork alike, is a `Derived` container that is a DIRECT child of its parent
 *    container (`amt.rs`: `parent_id = request.project_id.unwrap_or(0)`). Requests with no project land under the
 *    blueprint root, id 3 (`BLUEPRINT_ROOT_ID`, src/types/container.rs:27) — observed live: 10 AMT generations
 *    have `global_state.parent_id == 3` while `GetUserWorkspaces` returns no workspaces at all.
 *  - The first generation carries keyword `amt-main`; each later generation carries `amt-fork-of:<prior id>`.
 *  - TWO lineage mechanisms coexist and BOTH are read here:
 *      1. real `ForkOf` (child -> prior) / `ContinuedBy` (prior -> child) entries in
 *         `local_state.context.relationships` — only present on containers created after the lineage-mirroring
 *         fix (live: 40059 has ForkOf->30417, 30417 has ContinuedBy->40059);
 *      2. the older, one-directional `amt-fork-of:<id>` keyword — the ONLY lineage record on pre-fix forks
 *         (live: 30413, 30454, 30435, 30417, 40047, 30430, 40022, 40037 are keyword-only).
 *    `lineageSource` reports which mechanism(s) actually backed a generation's parent link — never inferred.
 *  - The host answers `/zsei/query` with an envelope `{success, result, error}` on both the Electron bridge and the
 *    HTTP fallback (main.js resolves the parsed body unchanged), so results are unwrapped defensively below.
 */
import { zseiQuery } from "../ozoneClient";

export interface AmtGeneration {
  containerId: number;
  /** The container this generation is a direct child of (a project id, or 3 for generations with no project). */
  projectId: number;
  /** Carries the `amt-main` keyword. */
  isMain: boolean;
  /** Parent generation. The real `ForkOf` edge wins over the keyword if both exist and disagree. */
  forkOf: number | null;
  /** Generations that continue this one: recorded `ContinuedBy` edges UNION the inverse of every other
   * generation's `forkOf`. See `continuedByEdgeIds` for the subset actually recorded on this container. */
  continuedBy: number[];
  /** Which mechanism(s) backed `forkOf`: "edge" (real ForkOf relation), "keyword" (legacy `amt-fork-of:` only),
   * "both", or "none" (no parent link recorded — expected for a main generation). */
  lineageSource: "edge" | "keyword" | "both" | "none";
  name?: string;
  keywords: string[];
  // ── additive fields (all optional so existing consumers are unaffected) ──
  /** Parent id from a real `ForkOf` relation on this container, if any. */
  forkOfEdgeId?: number | null;
  /** Parent id parsed from this container's `amt-fork-of:<id>` keyword, if any. */
  forkOfKeywordId?: number | null;
  /** Targets of `ContinuedBy` relations actually recorded on this container. */
  continuedByEdgeIds?: number[];
  /** True when an edge and a keyword both exist but name different parents. */
  lineageConflict?: boolean;
  /** True when `forkOf` names a container that is not among the scanned generations. */
  danglingParent?: boolean;
  /** True on an `amt-main` generation that is not the canonical (lowest-id) main of its parent container. */
  isDuplicateMain?: boolean;
}

/** Parent container of AMT generations created with no project id (BLUEPRINT_ROOT_ID, src/types/container.rs:27). */
export const UNATTACHED_AMT_PARENT_ID = 3;

// ── query plumbing ──────────────────────────────────────────────────────────

/** Unwrap the host envelope `{success, result, error}`; tolerate an already-unwrapped result. */
function unwrap(raw: unknown): any {
  if (raw && typeof raw === "object" && "success" in (raw as object) && "result" in (raw as object)) {
    const env = raw as { success: boolean; result: unknown; error?: string | null };
    if (env.success === false) throw new Error(env.error || "zsei query failed");
    return env.result;
  }
  return raw;
}

async function getContainer(containerId: number): Promise<any | null> {
  const res = unwrap(await zseiQuery<unknown>({ GetContainer: { container_id: containerId } }));
  return res && typeof res === "object" && "Container" in res ? (res as any).Container : null;
}

/** Small worker pool — a container can have hundreds of children; don't fire them all at once. */
async function mapPool<T, R>(items: T[], limit: number, fn: (item: T) => Promise<R>): Promise<R[]> {
  const out: R[] = new Array(items.length);
  let next = 0;
  const workers = Array.from({ length: Math.min(limit, items.length) }, async () => {
    while (next < items.length) {
      const i = next++;
      out[i] = await fn(items[i]);
    }
  });
  await Promise.all(workers);
  return out;
}

const FORK_KEYWORD_PREFIX = "amt-fork-of:";

function parseForkKeyword(keywords: string[]): number | null {
  const kw = keywords.find((k) => k.startsWith(FORK_KEYWORD_PREFIX));
  if (!kw) return null;
  const id = Number(kw.slice(FORK_KEYWORD_PREFIX.length));
  return Number.isFinite(id) ? id : null;
}

// ── public API ──────────────────────────────────────────────────────────────

/**
 * Every AMT generation directly under `projectId` (pass `UNATTACHED_AMT_PARENT_ID` for generations that were
 * created without a project). Sorted by container id. Throws on a real query failure — callers show the error;
 * an empty array means the scan succeeded and nothing exists.
 */
export async function loadAmtGenerations(projectId: number): Promise<AmtGeneration[]> {
  const parent = await getContainer(projectId);
  if (!parent) throw new Error(`Container ${projectId} not found`);
  const childIds: number[] = parent.global_state?.child_ids ?? [];

  const children = await mapPool(childIds, 8, async (id) => ({ id, container: await getContainer(id) }));

  const gens: AmtGeneration[] = [];
  for (const { id, container } of children) {
    if (!container) continue;
    const ls = container.local_state ?? {};
    if (ls.metadata?.container_type !== "Derived") continue;
    const keywords: string[] = ls.context?.keywords ?? [];
    const isMain = keywords.includes("amt-main");
    const forkKeywordId = parseForkKeyword(keywords);
    if (!isMain && forkKeywordId === null) continue; // some other Derived container (e.g. an insight), not an AMT

    const rels: any[] = ls.context?.relationships ?? [];
    const forkOfEdge = rels.find((r) => r.relation_type === "ForkOf");
    const forkOfEdgeId: number | null = forkOfEdge ? Number(forkOfEdge.target_id) : null;
    const continuedByEdgeIds = rels
      .filter((r) => r.relation_type === "ContinuedBy")
      .map((r) => Number(r.target_id))
      .sort((a, b) => a - b);

    const forkOf = forkOfEdgeId ?? forkKeywordId;
    const lineageSource: AmtGeneration["lineageSource"] =
      forkOfEdgeId !== null && forkKeywordId !== null
        ? "both"
        : forkOfEdgeId !== null
          ? "edge"
          : forkKeywordId !== null
            ? "keyword"
            : "none";

    gens.push({
      containerId: id,
      projectId,
      isMain,
      forkOf,
      continuedBy: [],
      lineageSource,
      name: ls.metadata?.name ?? undefined,
      keywords,
      forkOfEdgeId,
      forkOfKeywordId: forkKeywordId,
      continuedByEdgeIds,
      lineageConflict: forkOfEdgeId !== null && forkKeywordId !== null && forkOfEdgeId !== forkKeywordId,
    });
  }
  gens.sort((a, b) => a.containerId - b.containerId);

  // continuedBy = recorded ContinuedBy edges ∪ inverse of every generation's forkOf (source kept honest via
  // continuedByEdgeIds), and dangling-parent / duplicate-main flags.
  const byId = new Map(gens.map((g) => [g.containerId, g]));
  const continued = new Map<number, Set<number>>();
  for (const g of gens) continued.set(g.containerId, new Set(g.continuedByEdgeIds ?? []));
  for (const g of gens) {
    if (g.forkOf !== null) continued.get(g.forkOf)?.add(g.containerId);
    g.danglingParent = g.forkOf !== null && !byId.has(g.forkOf);
  }
  for (const g of gens) g.continuedBy = Array.from(continued.get(g.containerId) ?? []).sort((a, b) => a - b);

  const mains = gens.filter((g) => g.isMain);
  if (mains.length > 1) {
    const canonical = Math.min(...mains.map((m) => m.containerId));
    for (const m of mains) m.isDuplicateMain = m.containerId !== canonical;
  }
  return gens;
}

/** The canonical `amt-main` generation (lowest container id among mains), or null if none. */
export function canonicalMain(generations: AmtGeneration[]): AmtGeneration | null {
  const mains = generations.filter((g) => g.isMain);
  if (mains.length === 0) return null;
  return mains.reduce((a, b) => (a.containerId <= b.containerId ? a : b));
}

/** Direct children (forks) of every generation, keyed by parent container id. */
export function childrenByParent(generations: AmtGeneration[]): Map<number, AmtGeneration[]> {
  const map = new Map<number, AmtGeneration[]>();
  for (const g of generations) {
    if (g.forkOf === null) continue;
    const list = map.get(g.forkOf) ?? [];
    list.push(g);
    map.set(g.forkOf, list);
  }
  for (const list of map.values()) list.sort((a, b) => a.containerId - b.containerId);
  return map;
}

/**
 * Chain from `containerId` back toward its main generation, nearest first (the container itself is first).
 * Follows `forkOf`; stops at a generation with no `forkOf`, at a parent that isn't among `generations`
 * (dangling — chain simply ends there), or on a cycle. Returns [] if `containerId` isn't in `generations`.
 */
export function walkLineage(generations: AmtGeneration[], containerId: number): AmtGeneration[] {
  const byId = new Map(generations.map((g) => [g.containerId, g]));
  const chain: AmtGeneration[] = [];
  const seen = new Set<number>();
  let cur = byId.get(containerId);
  while (cur && !seen.has(cur.containerId)) {
    chain.push(cur);
    seen.add(cur.containerId);
    cur = cur.forkOf !== null ? byId.get(cur.forkOf) : undefined;
  }
  return chain;
}
