/**
 * MonitoringPanel — the Ozone-Studio monitoring surface.
 *
 * Connected agents (live heartbeat, roles) + the activity feed, from
 * GET /monitor/summary. Rendered on the shared panel design system.
 */
import React, { useEffect, useState } from "react";
import { ActivityEvent, AgentInfo, fetchMonitorSummary } from "../ozoneClient";

interface MonitorSummary {
  agents: AgentInfo[];
  activity: ActivityEvent[];
}

const KIND_ICONS: Record<string, string> = {
  log: "📝",
  agent: "🔌",
  tool: "🔧",
  job: "⚙️",
  bridge: "🌉",
  external: "🌐",
};

function seenLabel(registeredAt: number): string {
  const s = Math.max(0, Math.floor(Date.now() / 1000 - registeredAt));
  if (s < 5) return "now";
  if (s < 60) return `${s}s ago`;
  if (s < 3600) return `${Math.floor(s / 60)}m ago`;
  return `${Math.floor(s / 3600)}h ago`;
}

function timeLabel(ts: number): string {
  return new Date(ts * 1000).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

export const MonitoringPanel: React.FC<{ pollMs?: number }> = ({
  pollMs = 4000,
}) => {
  const [summary, setSummary] = useState<MonitorSummary | null>(null);
  const [connected, setConnected] = useState(false);
  const [filter, setFilter] = useState<string>("all");

  useEffect(() => {
    let cancelled = false;
    const poll = async () => {
      try {
        const result = await fetchMonitorSummary();
        if (cancelled) return;
        setSummary(result);
        setConnected(true);
      } catch {
        if (!cancelled) setConnected(false);
      }
    };
    poll();
    const interval = setInterval(poll, pollMs);
    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, [pollMs]);

  const agents = summary?.agents ?? [];
  const activity = (summary?.activity ?? []).filter(
    (e) => filter === "all" || e.kind === filter,
  );

  return (
    <div className="opanel">
      <div className="opanel-head">
        <span className={`odot ${connected ? "ok" : "err"}`} />
        <span className="opanel-title">Ozone-Studio Monitor</span>
      </div>
      <p className="opanel-sub">
        One observable surface: every agent, tool, and event on this host.
      </p>

      <div className="ostats">
        <div className="ostat">
          <div className="ostat-num">{agents.length}</div>
          <div className="ostat-label">Agents</div>
        </div>
        <div className="ostat">
          <div className="ostat-num">{activity.length}</div>
          <div className="ostat-label">Events</div>
        </div>
        <div className="ostat">
          <div className="ostat-num">
            {agents.reduce((a, x) => a + (x.call_count ?? 0), 0)}
          </div>
          <div className="ostat-label">Dispatched calls</div>
        </div>
      </div>

      <h4 style={{ margin: "0 0 8px", color: "#aebdce", fontSize: 12, textTransform: "uppercase", letterSpacing: 0.7 }}>
        Connected Agents
      </h4>
      {agents.length === 0 ? (
        <div className="oempty">
          No agents registered yet — pipelines boot with <code>--serve</code>,
          <br />
          ZCode, the browser plugin, and phones register the same way.
        </div>
      ) : (
        <table className="otable">
          <thead>
            <tr>
              <th>ID</th>
              <th>Name</th>
              <th>Roles</th>
              <th>Heartbeat</th>
              <th>Calls</th>
              <th>Execute URL</th>
            </tr>
          </thead>
          <tbody>
            {agents.map((a) => {
              const fresh = Date.now() / 1000 - a.registered_at < 45;
              return (
                <tr key={a.pipeline_id}>
                  <td className="omono">{a.pipeline_id}</td>
                  <td style={{ color: "#e8eef6", fontWeight: 500 }}>{a.name}</td>
                  <td>
                    {(a.roles ?? ["agent"]).map((r) => (
                      <span key={r} className={`ochip ${r}`}>
                        {r}
                      </span>
                    ))}
                  </td>
                  <td>
                    <span
                      className={`odot ${fresh ? "ok" : "err"}`}
                      style={{ marginRight: 7, width: 7, height: 7 }}
                    />
                    {seenLabel(a.registered_at)}
                  </td>
                  <td className="omono">{a.call_count}</td>
                  <td className="omono">{a.execute_url}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}

      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          margin: "18px 0 8px",
        }}
      >
        <h4
          style={{
            margin: 0,
            color: "#aebdce",
            fontSize: 12,
            textTransform: "uppercase",
            letterSpacing: 0.7,
          }}
        >
          Activity Feed
        </h4>
        <div className="oseg">
          {["all", "agent", "tool", "job", "bridge", "external"].map((k) => (
            <button
              key={k}
              className={filter === k ? "on" : ""}
              onClick={() => setFilter(k)}
            >
              {k}
            </button>
          ))}
        </div>
      </div>
      {activity.length === 0 ? (
        <div className="oempty">No activity recorded.</div>
      ) : (
        <div className="ofeed">
          {activity.map((e) => (
            <div key={e.id} className={`ofeed-row ${e.level}`}>
              <span>{KIND_ICONS[e.kind] ?? "•"}</span>
              <span className="ofeed-src">{e.source}</span>
              <span>{e.message}</span>
              <span className="ofeed-time">{timeLabel(e.timestamp)}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

export default MonitoringPanel;
