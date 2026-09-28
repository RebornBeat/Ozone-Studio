/**
 * I2 — Live fork/agent activity indicator + Coordination-tab panel.
 *
 * Real data only, from the B10/B11 routes (`../../data/coordinationData`),
 * which read `.ozone-context/state.json` directly on the host — NOT the
 * ZSEI-mirrored coordination graph. Confirmed against the plan's own B11
 * finding: `file_release` doesn't mirror into the graph (so a graph-based
 * claims view would show every released file as permanently still-claimed),
 * but these routes bypass that entirely by reading the state file itself —
 * this fork is unaffected by that gap.
 *
 * Verified live (2026-09-27): `GET /coordination/presence` → `{live:[],
 * ttl_seconds:300}` (no agent currently sending presence heartbeats — the
 * MCP server only writes a session's `last_seen` on tool calls, there's no
 * periodic heartbeat, so "live" is realistically almost always empty outside
 * an agent's active tool-call moment). `GET /coordination/claims` → several
 * REAL entries, some with `age_min` in the thousands (5000+) — claims from
 * forks that finished without releasing. Both states (empty presence, stale
 * claims) are the honest current norm, not edge cases — designed for below.
 */
import React, { useEffect, useState } from "react";
import { fetchPresence, fetchClaims, PresenceEntry, FileClaim } from "../../data/coordinationData";

const POLL_MS = 5000; // matches this codebase's existing TaskDetailPanel.tsx polling cadence order-of-magnitude (2000ms), slightly relaxed since this isn't a running task.

function formatAge(ageMin: number): string {
  if (ageMin < 1) return "just now";
  if (ageMin < 60) return `${ageMin}m ago`;
  const h = Math.floor(ageMin / 60);
  if (h < 24) return `${h}h ago`;
  return `${Math.floor(h / 24)}d ago`;
}

function useCoordinationPoll() {
  const [live, setLive] = useState<PresenceEntry[]>([]);
  const [ttlSeconds, setTtlSeconds] = useState(300);
  const [claims, setClaims] = useState<FileClaim[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    let cancelled = false;
    async function poll() {
      try {
        const [p, c] = await Promise.all([fetchPresence(), fetchClaims()]);
        if (cancelled) return;
        setLive(p.live);
        setTtlSeconds(p.ttl_seconds);
        setClaims(c.claims);
        setError(null);
      } catch (err) {
        if (!cancelled) setError(err instanceof Error ? err.message : String(err));
      } finally {
        if (!cancelled) setLoaded(true);
      }
    }
    poll();
    const interval = setInterval(poll, POLL_MS);
    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, []);

  return { live, ttlSeconds, claims, error, loaded };
}

/** Compact, persistent, non-intrusive — mounted in MetaPortion's chat header.
 * Renders NOTHING when there's real nothing to show (no fake "0 agents" state,
 * no error toast for a transient poll failure — the panel below is where a
 * real error belongs, not the header). */
export const AgentActivityIndicator: React.FC = () => {
  const { live, loaded } = useCoordinationPoll();
  if (!loaded || live.length === 0) return null;
  return (
    <span
      title={live.map((a) => `${a.agent} (${a.role || "agent"})${a.task ? ` — ${a.task}` : ""}`).join("\n")}
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: 5,
        fontSize: 11,
        color: "#8fe38f",
        marginLeft: 10,
        padding: "2px 8px",
        borderRadius: 999,
        border: "1px solid #1e2836",
        background: "rgba(143,227,143,0.08)",
      }}
    >
      <span style={{ width: 6, height: 6, borderRadius: "50%", background: "#8fe38f", display: "inline-block" }} />
      {live.length} agent{live.length === 1 ? "" : "s"} active
    </span>
  );
};

const C_TEXT = "#dfe7f2";
const C_BODY = "#c7d0dc";
const C_MUTED = "#8b98ab";
const C_BORDER = "#1e2836";
const C_WARN = "#e8c14f";

/** Full Coordination-tab view: live agents + active claims. `projectId` isn't
 * used — presence/claims are workspace-wide, not project-scoped (verified:
 * neither route accepts or returns a project filter). */
export const AgentActivityPanel: React.FC<{ projectId: number | null }> = () => {
  const { live, ttlSeconds, claims, error, loaded } = useCoordinationPoll();

  return (
    <div>
      <div style={{ fontSize: 12.5, fontWeight: 700, color: C_TEXT, marginBottom: 6 }}>Live agents</div>
      <div style={{ fontSize: 11, color: C_MUTED, marginBottom: 8 }}>
        Live = a presence event within the last {Math.round(ttlSeconds / 60)} minutes (the same window the
        coordination server itself uses). This session's own agent only reports on an active tool call, so an empty
        list here is the normal state between calls, not necessarily "no one is working."
      </div>
      {error && <div style={{ fontSize: 12, color: "#ff8a8a", marginBottom: 8 }}>Poll error: {error}</div>}
      {!loaded && !error && <div style={{ fontSize: 12, color: C_MUTED }}>Loading…</div>}
      {loaded && live.length === 0 && (
        <div style={{ fontSize: 12, color: C_MUTED, marginBottom: 16 }}>No agent currently live.</div>
      )}
      {live.length > 0 && (
        <div style={{ marginBottom: 16, display: "flex", flexDirection: "column", gap: 6 }}>
          {live.map((a) => (
            <div
              key={a.agent}
              style={{ border: `1px solid ${C_BORDER}`, borderRadius: 8, padding: "8px 10px", fontSize: 12 }}
            >
              <div style={{ color: C_TEXT, fontWeight: 600 }}>
                {a.agent} <span style={{ color: C_MUTED, fontWeight: 400 }}>{a.role || "(no role given)"}</span>
              </div>
              {a.task && <div style={{ color: C_BODY, marginTop: 2 }}>{a.task}</div>}
              {a.current_files.length > 0 && (
                <div style={{ color: C_MUTED, marginTop: 2, overflowWrap: "anywhere" }}>
                  {a.current_files.join(", ")}
                </div>
              )}
              <div style={{ color: C_MUTED, marginTop: 2 }}>seen {formatAge(Math.round(a.last_seen_age_s / 60))}</div>
            </div>
          ))}
        </div>
      )}

      <div style={{ fontSize: 12.5, fontWeight: 700, color: C_TEXT, marginBottom: 6, paddingTop: 8, borderTop: `1px solid ${C_BORDER}` }}>
        Active file claims
      </div>
      {loaded && claims.length === 0 && <div style={{ fontSize: 12, color: C_MUTED }}>No files currently claimed.</div>}
      {claims.length > 0 && (
        <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
          {claims
            .slice()
            .sort((a, b) => a.age_min - b.age_min)
            .map((c) => {
              // A claim older than the live TTL almost certainly belongs to an
              // agent that finished without releasing (verified live: several
              // real claims sit at 5000+ minutes) — flag it visibly rather
              // than presenting it as if someone is actively editing right now.
              const stale = c.age_min > ttlSeconds / 60;
              return (
                <div
                  key={c.file}
                  style={{
                    fontSize: 11.5,
                    padding: "4px 8px",
                    borderRadius: 6,
                    border: `1px solid ${C_BORDER}`,
                    opacity: stale ? 0.7 : 1,
                  }}
                >
                  <div style={{ color: C_BODY, overflowWrap: "anywhere" }}>{c.file}</div>
                  <div style={{ color: C_MUTED, display: "flex", gap: 8, marginTop: 1 }}>
                    <span>{c.agent ?? "(no agent recorded)"}</span>
                    <span>{formatAge(c.age_min)}</span>
                    {stale && <span style={{ color: C_WARN }}>likely stale</span>}
                  </div>
                  {c.reason && <div style={{ color: C_MUTED, marginTop: 1, overflowWrap: "anywhere" }}>{c.reason}</div>}
                </div>
              );
            })}
        </div>
      )}
    </div>
  );
};

export default AgentActivityPanel;
