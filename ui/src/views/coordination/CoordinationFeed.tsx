/**
 * I1 — Coordination event feed. Real `CoordinationEvent` containers via
 * `../../data/coordinationEvents` (`SHARED_CONTEXT_ROOT_ID`, real, live —
 * 260 real events as of 2026-09-27: notes/decisions/handoffs/findings/claims
 * from both this session and ZCode). No mock data on any path; an empty
 * result (e.g. a scope with nothing in it) renders an honest empty state.
 */
import React, { useEffect, useMemo, useState } from "react";
import { zseiQuery } from "../../ozoneClient";
import {
  CoordinationEvent,
  loadCoordinationEventBody,
  loadCoordinationEvents,
} from "../../data/coordinationEvents";

export type CoordinationFeedProps = { projectId: number | null };

const C_TEXT = "#dfe7f2";
const C_BODY = "#c7d0dc";
const C_MUTED = "#8b98ab";
const C_BORDER = "#1e2836";

const KIND_COLOR: Record<string, string> = {
  note: "#5fb3ff",
  decision: "#e8c14f",
  handoff: "#8fe38f",
  finding: "#ff8a8a",
  claim: "#b48cff",
};

function kindColor(kind: string): string {
  return KIND_COLOR[kind] ?? "#9aa5b5";
}

/** This project's real workspace id, resolved from its container's real
 * `global_state.parent_id` (the same parent/child convention every other
 * project-scoped fetch in this app relies on) — used only to offer a
 * "This project" / "This workspace" scope filter; falls back to no
 * resolution (filters just stay unavailable) on any failure. */
function useProjectScope(projectId: number | null): { wsKeyword: string | null; projKeyword: string | null } {
  const [wsKeyword, setWsKeyword] = useState<string | null>(null);
  useEffect(() => {
    let cancelled = false;
    if (projectId === null) {
      setWsKeyword(null);
      return;
    }
    zseiQuery<any>({ GetContainer: { container_id: projectId } })
      .then((r) => {
        if (cancelled) return;
        const parentId = r?.Container?.global_state?.parent_id;
        setWsKeyword(typeof parentId === "number" ? `ws:${parentId}` : null);
      })
      .catch(() => !cancelled && setWsKeyword(null));
    return () => {
      cancelled = true;
    };
  }, [projectId]);
  return { wsKeyword, projKeyword: projectId !== null ? `proj:${projectId}` : null };
}

const EventRow: React.FC<{ event: CoordinationEvent }> = ({ event }) => {
  const [expanded, setExpanded] = useState(false);
  const [body, setBody] = useState<{ body: string; detail: unknown } | null>(null);
  const [loadingBody, setLoadingBody] = useState(false);
  const [bodyError, setBodyError] = useState<string | null>(null);

  function toggle() {
    const next = !expanded;
    setExpanded(next);
    if (next && body === null && !loadingBody) {
      setLoadingBody(true);
      setBodyError(null);
      loadCoordinationEventBody(event.containerId)
        .then((b) => setBody(b ?? { body: "", detail: null }))
        .catch((e) => setBodyError(e instanceof Error ? e.message : String(e)))
        .finally(() => setLoadingBody(false));
    }
  }

  return (
    <div style={{ borderBottom: `1px solid ${C_BORDER}`, padding: "8px 4px" }}>
      <div
        onClick={toggle}
        style={{ display: "flex", alignItems: "baseline", gap: 8, cursor: "pointer" }}
      >
        <span
          style={{
            fontSize: 10.5,
            fontWeight: 700,
            textTransform: "uppercase",
            color: kindColor(event.kind),
            border: `1px solid ${kindColor(event.kind)}`,
            borderRadius: 4,
            padding: "1px 5px",
            flexShrink: 0,
          }}
        >
          {event.kind}
        </span>
        <span style={{ fontSize: 12, color: C_TEXT, flex: 1, overflowWrap: "anywhere" }}>{event.title}</span>
        <span style={{ fontSize: 11, color: C_MUTED, flexShrink: 0 }}>{event.agent}</span>
        <span style={{ fontSize: 10.5, color: C_MUTED, flexShrink: 0 }}>
          {event.mirroredAt ? new Date(event.mirroredAt).toLocaleString() : "unknown time"}
        </span>
      </div>
      {expanded && (
        <div style={{ marginTop: 6, marginLeft: 4, fontSize: 11.5, color: C_BODY }}>
          <div style={{ color: C_MUTED, marginBottom: 3 }}>
            scope: {event.scopeKeywords.join(", ") || "(none recorded)"}
            {event.files.length > 0 && <> · files: {event.files.join(", ")}</>}
          </div>
          {loadingBody && <div style={{ color: C_MUTED }}>Loading body…</div>}
          {bodyError && <div style={{ color: "#ff8a8a" }}>Error loading body: {bodyError}</div>}
          {body && (
            <>
              <div style={{ whiteSpace: "pre-wrap", overflowWrap: "anywhere" }}>
                {body.body || <span style={{ color: C_MUTED, fontStyle: "italic" }}>No body recorded.</span>}
              </div>
              {body.detail !== null && body.detail !== undefined && (
                <details style={{ marginTop: 4 }}>
                  <summary style={{ cursor: "pointer", color: C_MUTED }}>Raw detail JSON</summary>
                  <pre style={{ whiteSpace: "pre-wrap", overflowWrap: "anywhere", fontSize: 11 }}>
                    {JSON.stringify(body.detail, null, 2)}
                  </pre>
                </details>
              )}
            </>
          )}
        </div>
      )}
    </div>
  );
};

export const CoordinationFeed: React.FC<CoordinationFeedProps> = ({ projectId }) => {
  const [status, setStatus] = useState<
    { kind: "loading" } | { kind: "error"; message: string } | { kind: "ready"; events: CoordinationEvent[] }
  >({ kind: "loading" });
  const [scopeFilter, setScopeFilter] = useState<string>("all");
  const [kindFilter, setKindFilter] = useState<string>("all");
  const [agentFilter, setAgentFilter] = useState<string>("all");
  const [search, setSearch] = useState("");
  const [visibleCount, setVisibleCount] = useState(50);
  const { wsKeyword, projKeyword } = useProjectScope(projectId);

  useEffect(() => {
    let cancelled = false;
    setStatus({ kind: "loading" });
    loadCoordinationEvents()
      .then((events) => !cancelled && setStatus({ kind: "ready", events }))
      .catch((e) => !cancelled && setStatus({ kind: "error", message: e instanceof Error ? e.message : String(e) }));
    return () => {
      cancelled = true;
    };
  }, []);

  const allEvents = status.kind === "ready" ? status.events : [];
  const kinds = useMemo(() => Array.from(new Set(allEvents.map((e) => e.kind))).sort(), [allEvents]);
  const agents = useMemo(() => Array.from(new Set(allEvents.map((e) => e.agent))).sort(), [allEvents]);

  const filtered = useMemo(() => {
    let list = allEvents;
    if (scopeFilter === "global") list = list.filter((e) => e.scopeKeywords.includes("scope:global"));
    else if (scopeFilter === "workspace" && wsKeyword)
      list = list.filter((e) => e.scopeKeywords.includes(wsKeyword));
    else if (scopeFilter === "project" && projKeyword)
      list = list.filter((e) => e.scopeKeywords.includes(projKeyword));
    if (kindFilter !== "all") list = list.filter((e) => e.kind === kindFilter);
    if (agentFilter !== "all") list = list.filter((e) => e.agent === agentFilter);
    if (search.trim()) {
      const q = search.trim().toLowerCase();
      list = list.filter((e) => e.title.toLowerCase().includes(q) || e.agent.toLowerCase().includes(q));
    }
    return list;
  }, [allEvents, scopeFilter, kindFilter, agentFilter, search, wsKeyword, projKeyword]);

  const selectStyle: React.CSSProperties = {
    background: "#101724",
    color: C_TEXT,
    border: `1px solid ${C_BORDER}`,
    borderRadius: 6,
    padding: "3px 7px",
    fontSize: 12,
  };

  return (
    <div style={{ fontSize: 12.5 }}>
      <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center", marginBottom: 10 }}>
        <select value={scopeFilter} onChange={(e) => setScopeFilter(e.target.value)} style={selectStyle}>
          <option value="all">All scopes</option>
          <option value="global">Global only</option>
          <option value="workspace" disabled={!wsKeyword}>
            This workspace{!wsKeyword && projectId !== null ? " (resolving…)" : ""}
          </option>
          <option value="project" disabled={!projKeyword}>
            This project
          </option>
        </select>
        <select value={kindFilter} onChange={(e) => setKindFilter(e.target.value)} style={selectStyle}>
          <option value="all">All kinds</option>
          {kinds.map((k) => (
            <option key={k} value={k}>
              {k}
            </option>
          ))}
        </select>
        <select value={agentFilter} onChange={(e) => setAgentFilter(e.target.value)} style={selectStyle}>
          <option value="all">All agents</option>
          {agents.map((a) => (
            <option key={a} value={a}>
              {a}
            </option>
          ))}
        </select>
        <input
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="Search title/agent…"
          style={{ ...selectStyle, flex: "1 1 160px" }}
        />
      </div>

      {status.kind === "loading" && <div style={{ color: C_MUTED, padding: 12 }}>Loading coordination events…</div>}
      {status.kind === "error" && (
        <div style={{ color: "#ff8a8a", padding: 12 }}>Error loading events: {status.message}</div>
      )}
      {status.kind === "ready" && (
        <>
          <div style={{ color: C_MUTED, fontSize: 11, marginBottom: 6 }}>
            {filtered.length} of {allEvents.length} real events
          </div>
          {filtered.length === 0 ? (
            <div style={{ color: C_MUTED, padding: 12 }}>No coordination events match this filter.</div>
          ) : (
            <>
              {filtered.slice(0, visibleCount).map((e) => (
                <EventRow key={e.containerId} event={e} />
              ))}
              {filtered.length > visibleCount && (
                <button
                  onClick={() => setVisibleCount((n) => n + 50)}
                  style={{ ...selectStyle, marginTop: 8, cursor: "pointer" }}
                >
                  Load {Math.min(50, filtered.length - visibleCount)} more
                </button>
              )}
            </>
          )}
        </>
      )}
    </div>
  );
};

export default CoordinationFeed;
