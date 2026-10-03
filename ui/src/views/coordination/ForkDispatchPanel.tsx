/**
 * I4 — Fork dispatch visualization.
 *
 * INVESTIGATED FIRST, per directive: no live "fork running/completed/failed"
 * signal is reachable from the frontend. Grepped `src/grpc/mod.rs`'s full
 * route table — the only task/coordination routes are `/task/{get,list,
 * cancel,step/rerun,create,update}` and `/coordination/{presence,claims}`
 * (B10/B11). None of these carry per-fork dispatch state. Subagent
 * task-notifications are internal to the coordinating CLI session and are
 * never persisted to any host-reachable store.
 *
 * The one REAL signal that exists: coordination-event containers under
 * `SHARED_CONTEXT_ROOT_ID=8` (B9), confirmed live — `POST /zsei/query
 * {"GetContainer":{"container_id":8}}` returns 260 real children today.
 * Each carries the note title verbatim as `local_state.metadata.name`
 * (e.g. "H2: pipeline execution list per step done — ...", "claim: ui/src/
 * views/coordination/AgentActivity.tsx") and real keywords (kind, agent,
 * `ws:<id>`, `file:<path>`/`claim:<path>` for claims). This session's own
 * forks consistently title their handoffs `"<ID>: <summary>"` (C8/D4/E2/H2/
 * etc) — that prefix is the only real "which fork" signal, extracted here
 * with a plain regex, not a purpose-built field.
 *
 * This view is therefore an honest "recent fork/agent activity" feed built
 * from real handoff/finding events and real file-claim events — NOT a live
 * dispatch tracker. It never shows "running" (no such state is observable)
 * and never invents a status a container doesn't actually carry.
 */
import React, { useEffect, useState } from "react";
import { zseiQuery } from "../../ozoneClient";

const COORDINATION_ROOT_ID = 8;

interface RawContainer {
  global_state: { child_ids: number[] };
  local_state: {
    metadata: { container_type: string; name?: string | null };
    context: { keywords: string[] };
  };
}

type ActivityKind = "handoff" | "finding" | "claim" | "release" | "other";

interface ForkActivityItem {
  containerId: number;
  kind: ActivityKind;
  agent: string;
  forkId: string | null; // e.g. "C8", "H2" — parsed from the title, may be absent
  title: string;
  file: string | null; // for claim/release events
}

async function getContainer(containerId: number): Promise<RawContainer | null> {
  const result = await zseiQuery<any>({ GetContainer: { container_id: containerId } });
  return result && typeof result === "object" && "Container" in result ? (result.Container as RawContainer) : null;
}

const FORK_ID_RE = /^([A-Z]\d{1,2}(?:-[A-Z]\d{1,2})?)\s*[:—-]/; // "C8:", "H2 —", "C8-C10:"
const FILE_KEYWORD_RE = /^(?:claim|file):(.+)$/;

function classifyKind(keywords: string[]): ActivityKind {
  if (keywords.includes("handoff")) return "handoff";
  if (keywords.includes("finding")) return "finding";
  if (keywords.includes("release")) return "release";
  if (keywords.includes("claim")) return "claim";
  return "other";
}

function agentOf(keywords: string[]): string {
  // Real convention (context_mirror.rs): keywords are [kind, agent, ...scope].
  // agent is whatever isn't a known kind/scope-prefixed/file-prefixed token.
  const known = new Set(["handoff", "finding", "claim", "release", "note", "decision"]);
  const candidate = keywords.find(
    (k) => !known.has(k) && !k.startsWith("ws:") && !k.startsWith("proj:") && !k.startsWith("scope:") && !FILE_KEYWORD_RE.test(k),
  );
  return candidate ?? "unknown";
}

function fileOf(keywords: string[]): string | null {
  for (const k of keywords) {
    const m = FILE_KEYWORD_RE.exec(k);
    if (m) return m[1];
  }
  return null;
}

const KIND_COLOR: Record<ActivityKind, string> = {
  handoff: "#5fb3ff",
  finding: "#ffb95f",
  claim: "#8fe38f",
  release: "var(--color-text-muted)",
  other: "#6b7a90",
};

export const ForkDispatchPanel: React.FC<{ projectId: number | null }> = () => {
  const [items, setItems] = useState<ForkActivityItem[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [kindFilter, setKindFilter] = useState<ActivityKind | "all">("all");

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const root = await getContainer(COORDINATION_ROOT_ID);
        if (!root) throw new Error(`Coordination root container ${COORDINATION_ROOT_ID} not found`);
        const ids = root.global_state.child_ids;
        // Real ids are monotonically-increasing timestamps-derived — take the
        // most recent slice rather than fetching all (260 today and growing).
        const recent = ids.slice(-60);
        const containers = await Promise.all(recent.map((id) => getContainer(id)));
        const parsed: ForkActivityItem[] = [];
        containers.forEach((c, i) => {
          if (!c || c.local_state.metadata.container_type !== "CoordinationEvent") return;
          const keywords = c.local_state.context.keywords ?? [];
          const title = c.local_state.metadata.name ?? "(untitled)";
          const forkMatch = FORK_ID_RE.exec(title);
          parsed.push({
            containerId: recent[i],
            kind: classifyKind(keywords),
            agent: agentOf(keywords),
            forkId: forkMatch ? forkMatch[1] : null,
            title,
            file: fileOf(keywords),
          });
        });
        parsed.reverse(); // newest first (ids were ascending)
        if (!cancelled) setItems(parsed);
      } catch (err) {
        if (!cancelled) setError(err instanceof Error ? err.message : String(err));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const filtered = items?.filter((it) => kindFilter === "all" || it.kind === kindFilter) ?? [];

  return (
    <div>
      <div
        style={{
          fontSize: 11.5,
          color: "var(--color-text-muted)",
          border: "1px solid var(--color-border-faint)",
          borderRadius: 6,
          padding: "6px 8px",
          marginBottom: 10,
          lineHeight: 1.5,
        }}
      >
        No live fork-status feed exists on this host — task-notifications never leave the coordinating CLI
        session, and no route exposes running/completed/failed state. This is a real activity feed built from
        coordination-event handoffs and file-claim events under container {COORDINATION_ROOT_ID}; a "fork id"
        (e.g. C8, H2) is parsed from the handoff title's own convention, not a tracked field.
      </div>

      <div style={{ display: "flex", gap: 6, marginBottom: 10, flexWrap: "wrap" }}>
        {(["all", "handoff", "finding", "claim", "release", "other"] as const).map((k) => (
          <button
            key={k}
            onClick={() => setKindFilter(k)}
            style={{
              background: kindFilter === k ? "var(--color-border-faint)" : "transparent",
              color: kindFilter === k ? "var(--color-text)" : "var(--color-text-muted)",
              border: "1px solid var(--color-border-faint)",
              borderRadius: 6,
              padding: "3px 9px",
              fontSize: 11.5,
              cursor: "pointer",
            }}
          >
            {k}
          </button>
        ))}
      </div>

      {error && <div style={{ color: "#ff8a8a", fontSize: 12.5 }}>Error: {error}</div>}
      {!error && items === null && <div style={{ color: "var(--color-text-muted)", fontSize: 12.5 }}>Loading…</div>}
      {!error && items !== null && filtered.length === 0 && (
        <div style={{ color: "var(--color-text-muted)", fontSize: 12.5 }}>No matching coordination activity in the most recent window.</div>
      )}
      {filtered.length > 0 && (
        <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
          {filtered.map((it) => (
            <div
              key={it.containerId}
              style={{
                border: "1px solid var(--color-border-faint)",
                borderLeft: `3px solid ${KIND_COLOR[it.kind]}`,
                borderRadius: 6,
                padding: "6px 10px",
                fontSize: 12,
                color: "var(--color-text-secondary)",
              }}
            >
              <div style={{ display: "flex", gap: 8, alignItems: "baseline" }}>
                {it.forkId && (
                  <span style={{ color: KIND_COLOR[it.kind], fontWeight: 700, fontSize: 11 }}>{it.forkId}</span>
                )}
                <span style={{ color: "var(--color-text-muted)", fontSize: 10.5, textTransform: "uppercase" }}>{it.kind}</span>
                <span style={{ color: "var(--color-text-muted)", fontSize: 10.5 }}>{it.agent}</span>
              </div>
              <div style={{ marginTop: 2, whiteSpace: "pre-wrap" }}>{it.title}</div>
              {it.file && <div style={{ marginTop: 2, color: "var(--color-text-muted)", fontSize: 11 }}>{it.file}</div>}
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

export default ForkDispatchPanel;
