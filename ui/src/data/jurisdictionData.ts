/**
 * Jurisdiction scope graph (B14) — the real "meta workspace" of jurisdiction
 * rule sets and the `RelatedTo` edges layering national scopes on the EU /
 * global baselines.
 *
 * Real sources (verified against the live host 2026-09-24):
 *  - JURISDICTION_ROOT_ID = 7 (src/types/container.rs). Its `global_state.child_ids`
 *    are the scope containers (src/orchestrator/jurisdiction_search.rs:237 and
 *    the bootstrap registration in src/lib.rs:526-625 both parent under it).
 *  - A scope is a `JurisdictionRuleSet` container. Region key = `keywords[0]`
 *    (both creation paths seed it with the scope key); the path
 *    `jurisdiction/{national/}{key}.json` in `object_store_path` is the fallback.
 *    Names differ by creation path ("Jurisdiction: xx" from bootstrap,
 *    "<Country> Jurisdiction Rules (...)" from jurisdiction_search), so several
 *    region keys have TWO containers — surfaced via `regionKeyShared`, never merged.
 *  - Edges: `local_state.context.relationships` entries of type `RelatedTo`
 *    (src/lib.rs ~651-690: every non-global scope -> global, EU members -> eu),
 *    always `discovered_via: "Manual"`. The `JurisdictionScope` relation type
 *    (=70) is declared but not wired anywhere — it is never special-cased here;
 *    whatever relation types really appear are passed through untouched.
 *
 * DATA-INTEGRITY FINDINGS (kept visible, not papered over): the root also holds
 * a large number of blank `Root`-type children (created_at 0, no name, no
 * keywords, no relationships — placeholders, not scopes), and most `RelatedTo`
 * targets point at those blanks rather than at a real rule set. Those relations
 * are returned with `targetResolved: false` so a consumer can count or omit
 * them, and both totals are exposed on `JurisdictionGraph`.
 *
 * `/zsei/query` answers `{success, result, error}`; neither the Electron bridge
 * nor the HTTP fallback unwraps it, so this module unwraps it itself (both the
 * wrapped and an already-unwrapped shape are accepted).
 */
import { zseiQuery } from "../ozoneClient";

export const JURISDICTION_ROOT_ID = 7;

export interface JurisdictionRelation {
  targetId: number;
  relationType: string;
  confidence: number;
  discoveredVia: string;
  /** Whether the target is itself a real JurisdictionRuleSet child of the root. */
  targetResolved: boolean;
  /** False when the stored relation carried no confidence (`confidence` is then 0 = "not recorded", not a measurement). */
  confidenceRecorded: boolean;
  /** False when the stored relation carried no `discovered_via` (`discoveredVia` is then ""). */
  discoveryRecorded: boolean;
  graphHops?: number;
  /** The untouched relation object as stored on the container. */
  raw: unknown;
}

export interface JurisdictionScope {
  containerId: number;
  regionKey: string;
  name: string;
  relations: JurisdictionRelation[];
  /** `metadata.provenance` as stored ("bootstrap", "jurisdiction_search", ...). */
  provenance?: string;
  objectStorePath?: string;
  /** More than one JurisdictionRuleSet container exists for this region key. */
  regionKeyShared: boolean;
  /** Outgoing relations whose target is not a real rule set. */
  unresolvedRelationCount: number;
}

export interface JurisdictionGraph {
  scopes: JurisdictionScope[];
  /** Root children that are not JurisdictionRuleSet containers (blank placeholders in the live data). */
  nonRuleSetChildCount: number;
  totalRelationCount: number;
  /** Relations whose target is not a real rule set. */
  danglingRelationCount: number;
}

// ── plumbing ────────────────────────────────────────────────────────────────

async function zsei<T = any>(query: Record<string, unknown>): Promise<T> {
  const res: any = await zseiQuery<any>(query);
  if (res && typeof res === "object" && "success" in res) {
    if (!res.success) throw new Error(`ZSEI query failed: ${res.error ?? "unknown error"}`);
    return res.result as T;
  }
  return res as T;
}

async function getContainer(id: number): Promise<any | null> {
  const r = await zsei<any>({ GetContainer: { container_id: id } });
  return r?.Container ?? null;
}

async function mapLimit<T, R>(items: T[], limit: number, fn: (t: T) => Promise<R>): Promise<R[]> {
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

function regionKeyOf(id: number, c: any): string {
  const kw: unknown = c?.local_state?.context?.keywords?.[0];
  if (typeof kw === "string" && kw) return kw;
  const path: unknown = c?.local_state?.storage?.object_store_path;
  if (typeof path === "string") {
    const base = path.split("/").pop()?.replace(/\.json$/, "");
    if (base) return base;
  }
  return `#${id}`;
}

// ── loader (memoised briefly: overlay, D5 and H5 all call it) ───────────────

const CACHE_MS = 15_000;
let cache: { at: number; promise: Promise<JurisdictionGraph> } | null = null;

export function loadJurisdictionGraph(force = false): Promise<JurisdictionGraph> {
  if (!force && cache && Date.now() - cache.at < CACHE_MS) return cache.promise;
  const promise = fetchGraph();
  cache = { at: Date.now(), promise };
  promise.catch(() => {
    if (cache?.promise === promise) cache = null;
  });
  return promise;
}

async function fetchGraph(): Promise<JurisdictionGraph> {
  const root = await getContainer(JURISDICTION_ROOT_ID);
  if (!root) throw new Error(`Jurisdiction root container ${JURISDICTION_ROOT_ID} not found`);
  const childIds: number[] = root.global_state?.child_ids ?? [];
  const children = await mapLimit(childIds, 16, async (id) => ({ id, c: await getContainer(id) }));

  const ruleSets = children.filter(
    (x): x is { id: number; c: any } => !!x.c && x.c.local_state?.metadata?.container_type === "JurisdictionRuleSet",
  );
  const ruleSetIds = new Set(ruleSets.map((x) => x.id));

  const keyCounts = new Map<string, number>();
  for (const { id, c } of ruleSets) {
    const k = regionKeyOf(id, c);
    keyCounts.set(k, (keyCounts.get(k) ?? 0) + 1);
  }

  let totalRelationCount = 0;
  let danglingRelationCount = 0;
  const scopes: JurisdictionScope[] = ruleSets.map(({ id, c }) => {
    const regionKey = regionKeyOf(id, c);
    const stored: any[] = c.local_state?.context?.relationships ?? [];
    const relations: JurisdictionRelation[] = stored.map((r) => {
      const targetResolved = ruleSetIds.has(r.target_id);
      totalRelationCount += 1;
      if (!targetResolved) danglingRelationCount += 1;
      const confidenceRecorded = typeof r.confidence === "number";
      const discoveryRecorded = typeof r.discovered_via === "string" && r.discovered_via !== "";
      return {
        targetId: r.target_id,
        relationType: String(r.relation_type),
        confidence: confidenceRecorded ? r.confidence : 0,
        discoveredVia: discoveryRecorded ? r.discovered_via : "",
        targetResolved,
        confidenceRecorded,
        discoveryRecorded,
        ...(typeof r.graph_hops === "number" ? { graphHops: r.graph_hops } : {}),
        raw: r,
      };
    });
    return {
      containerId: id,
      regionKey,
      name: c.local_state?.metadata?.name ?? regionKey,
      relations,
      provenance: c.local_state?.metadata?.provenance || undefined,
      objectStorePath: c.local_state?.storage?.object_store_path ?? undefined,
      regionKeyShared: (keyCounts.get(regionKey) ?? 0) > 1,
      unresolvedRelationCount: relations.filter((r) => !r.targetResolved).length,
    };
  });

  return {
    scopes,
    nonRuleSetChildCount: children.length - ruleSets.length,
    totalRelationCount,
    danglingRelationCount,
  };
}

/** Every real JurisdictionRuleSet scope under the jurisdiction root, with its outgoing relations. */
export async function loadJurisdictionScopes(): Promise<JurisdictionScope[]> {
  return (await loadJurisdictionGraph()).scopes;
}
