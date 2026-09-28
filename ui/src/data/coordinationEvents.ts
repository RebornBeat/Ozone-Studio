/**
 * Coordination event feed (I1) — real `CoordinationEvent` containers under
 * `SHARED_CONTEXT_ROOT_ID` (=8, `src/types/container.rs`), mirrored by
 * `src/context_mirror.rs::mirror()` for every real note/decision/handoff/
 * finding/claim (`tools/ozone-shared-context/server.js` calls this on every
 * `note_add`/`file_claim`/`file_release`).
 *
 * Real shape, confirmed live against the running host (260 real containers
 * as of 2026-09-27, not a guess):
 *   - `local_state.metadata.name`        = the real title
 *   - `local_state.metadata.provenance`  = the real agent, original case
 *   - `local_state.metadata.created_at`  = real unix-seconds mirror time
 *   - `local_state.context.keywords`     = [kind (lowercase), agent
 *     (lowercase), `file:<path>`... , scope keyword(s): `scope:global` |
 *     `ws:<id>` | `proj:<id>`, + a `claim:<path>` dedupe keyword for claims]
 *   - `local_state.storage.object_store_path` = `shared_context/<slug>.json`,
 *     the ONLY place `body`/`detail` live — `GetContainer` never returns
 *     them, so a row's full body needs one extra `GetContainerContent` call
 *     (B0, real) made lazily on expand, not eagerly for all 260 rows.
 *
 * `kind` is read from `keywords[0]` by construction (`context_mirror.rs`
 * pushes `kind.to_lowercase()` first, always) rather than re-deriving it —
 * confirmed against every sample checked live.
 */
import { zseiQuery } from "../ozoneClient";

export interface CoordinationEvent {
  containerId: number;
  kind: string;
  agent: string;
  title: string;
  body?: string;
  files: string[];
  scopeKeywords: string[];
  mirroredAt?: string;
}

export interface CoordinationEventQuery {
  limit?: number;
  kind?: string;
  scopeKeyword?: string;
}

interface ContainerJson {
  global_state?: { child_ids?: number[] };
  local_state?: {
    metadata?: { name?: string | null; provenance?: string; created_at?: number };
    context?: { keywords?: string[] };
  };
}

async function getContainer(containerId: number): Promise<ContainerJson | null> {
  const result = await zseiQuery<any>({ GetContainer: { container_id: containerId } });
  if (result && typeof result === "object" && "Container" in result) {
    return result.Container as ContainerJson;
  }
  return null;
}

/** Body/files/detail live only behind `object_store_path` — fetched lazily,
 * one container at a time, when a row is expanded. Real fields per
 * `context_mirror.rs::mirror()`'s persisted JSON: kind/agent/title/body/
 * files/detail/mirrored_at. */
export async function loadCoordinationEventBody(
  containerId: number,
): Promise<{ body: string; detail: unknown } | null> {
  const result = await zseiQuery<any>({ GetContainerContent: { container_id: containerId } });
  const content = result && typeof result === "object" ? result.Content : null;
  const json = content?.json;
  if (!json || typeof json !== "object") return null;
  return {
    body: typeof json.body === "string" ? json.body : "",
    detail: json.detail ?? null,
  };
}

const SHARED_CONTEXT_ROOT_ID = 8;

function isScopeKeyword(k: string): boolean {
  return k === "scope:global" || k.startsWith("ws:") || k.startsWith("proj:");
}

/** All real files referenced by an event, parsed from its `file:<path>` keywords. */
function filesFromKeywords(keywords: string[]): string[] {
  return keywords.filter((k) => k.startsWith("file:")).map((k) => k.slice("file:".length));
}

/**
 * Newest first. Fetches the full real event list under the SharedContext
 * root (260 real containers as of this writing — a real, bounded volume,
 * fetched in parallel batches rather than one call per row sequentially).
 * `body` is left undefined here (lazy — see `loadCoordinationEventBody`);
 * `opts.kind`/`opts.scopeKeyword` filter on the real keywords already
 * present on every row, no extra fetch needed. `opts.limit` caps the
 * RETURNED (already-sorted, already-filtered) list — it does not skip real
 * events, since the root's `child_ids` list carries no reliable order to
 * page against (confirmed live: ids are not chronological).
 */
export async function loadCoordinationEvents(
  opts: CoordinationEventQuery = {},
): Promise<CoordinationEvent[]> {
  const root = await getContainer(SHARED_CONTEXT_ROOT_ID);
  const childIds = root?.global_state?.child_ids ?? [];

  const BATCH = 40;
  const containers: (ContainerJson | null)[] = [];
  for (let i = 0; i < childIds.length; i += BATCH) {
    const batch = childIds.slice(i, i + BATCH);
    const results = await Promise.all(batch.map((id) => getContainer(id)));
    containers.push(...results);
  }

  const events: CoordinationEvent[] = [];
  childIds.forEach((id, i) => {
    const c = containers[i];
    if (!c) return; // honest skip: container vanished between list and fetch
    const meta = c.local_state?.metadata ?? {};
    const keywords = c.local_state?.context?.keywords ?? [];
    const kind = keywords[0] ?? "unknown";
    const agent = meta.provenance ?? keywords[1] ?? "unknown";
    events.push({
      containerId: id,
      kind,
      agent,
      title: meta.name ?? `(untitled ${kind})`,
      files: filesFromKeywords(keywords),
      scopeKeywords: keywords.filter(isScopeKeyword),
      mirroredAt:
        typeof meta.created_at === "number"
          ? new Date(meta.created_at * 1000).toISOString()
          : undefined,
    });
  });

  events.sort((a, b) => (b.mirroredAt ?? "").localeCompare(a.mirroredAt ?? ""));

  let filtered = events;
  if (opts.kind) filtered = filtered.filter((e) => e.kind === opts.kind);
  if (opts.scopeKeyword) filtered = filtered.filter((e) => e.scopeKeywords.includes(opts.scopeKeyword!));
  if (opts.limit !== undefined) filtered = filtered.slice(0, opts.limit);
  return filtered;
}
