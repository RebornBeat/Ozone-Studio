/**
 * ConnectedAgents — live view of pipelines registered with the host
 * (the connect-model registry: GET /pipelines/remote via the preload bridge).
 *
 * Any pipeline that booted with --serve and announced itself appears here:
 * id, name, execute URL, registration time, and live call count. This is the
 * dashboard surface of src/pipeline/remote.rs.
 */
import React, { useEffect, useState } from "react";
import { AgentInfo, fetchRemotePipelines } from "../ozoneClient";

interface Props {
  /** Poll interval in ms (default 5s). */
  pollMs?: number;
  /** Compact mode for embedding in a status bar. */
  compact?: boolean;
}

/** "3s ago" style label — registered_at refreshes on every heartbeat. */
function lastSeen(registeredAt: number): string {
  const secs = Math.max(0, Math.floor(Date.now() / 1000 - registeredAt));
  if (secs < 5) return "now";
  if (secs < 60) return `${secs}s ago`;
  if (secs < 3600) return `${Math.floor(secs / 60)}m ago`;
  return `${Math.floor(secs / 3600)}h ago`;
}

function roleColor(role: string): string {
  switch (role) {
    case "model":
      return "#9b59b6";
    case "observer":
      return "#3498db";
    default:
      return "#2ecc71";
  }
}

export const ConnectedAgents: React.FC<Props> = ({
  pollMs = 5000,
  compact = false,
}) => {
  const [agents, setAgents] = useState<AgentInfo[]>([]);
  const [connected, setConnected] = useState(false);
  const [lastUpdate, setLastUpdate] = useState<Date | null>(null);

  useEffect(() => {
    let cancelled = false;

    const poll = async () => {
      try {
        const result = await fetchRemotePipelines();
        if (cancelled) return;
        setAgents(result?.pipelines ?? []);
        setConnected(true);
        setLastUpdate(new Date());
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

  const dot = (color: string) => (
    <span
      style={{
        display: "inline-block",
        width: 8,
        height: 8,
        borderRadius: "50%",
        background: color,
        marginRight: 6,
      }}
    />
  );

  if (compact) {
    return (
      <span title={`Connected pipelines: ${agents.length}`}>
        {dot(connected ? "#2ecc71" : "#95a5a6")}
        {agents.length} agent{agents.length === 1 ? "" : "s"}
      </span>
    );
  }

  return (
    <div className="connected-agents">
      <h3>
        {dot(connected ? "#2ecc71" : "#95a5a6")}
        Connected Pipelines
        <span style={{ opacity: 0.6, fontSize: 12, marginLeft: 8 }}>
          {lastUpdate ? `updated ${lastUpdate.toLocaleTimeString()}` : ""}
        </span>
      </h3>

      {agents.length === 0 ? (
        <p style={{ opacity: 0.6 }}>
          No pipelines registered. Boot a pipeline with{" "}
          <code>--serve --register http://&lt;host&gt;:&lt;port&gt;/pipelines/register</code>{" "}
          to see it here.
        </p>
      ) : (
        <table style={{ width: "100%", borderCollapse: "collapse" }}>
          <thead>
            <tr style={{ textAlign: "left", opacity: 0.7 }}>
              <th style={{ padding: 4 }}>ID</th>
              <th style={{ padding: 4 }}>Name</th>
              <th style={{ padding: 4 }}>Roles</th>
              <th style={{ padding: 4 }}>Heartbeat</th>
              <th style={{ padding: 4 }}>Calls</th>
              <th style={{ padding: 4 }}>Execute URL</th>
            </tr>
          </thead>
          <tbody>
            {agents.map((a) => (
              <tr key={a.pipeline_id}>
                <td style={{ padding: 4 }}>{a.pipeline_id}</td>
                <td style={{ padding: 4 }}>{a.name}</td>
                <td style={{ padding: 4 }}>
                  {(a.roles ?? ["agent"]).map((r) => (
                    <span
                      key={r}
                      style={{
                        display: "inline-block",
                        fontSize: 11,
                        padding: "1px 7px",
                        marginRight: 4,
                        borderRadius: 8,
                        color: roleColor(r),
                        border: `1px solid ${roleColor(r)}`,
                      }}
                    >
                      {r}
                    </span>
                  ))}
                </td>
                <td style={{ padding: 4 }}>
                  <span
                    style={{
                      display: "inline-block",
                      width: 7,
                      height: 7,
                      borderRadius: "50%",
                      background:
                        Date.now() / 1000 - a.registered_at < 45
                          ? "#2ecc71"
                          : "#f39c12",
                      marginRight: 6,
                    }}
                  />
                  {lastSeen(a.registered_at)}
                </td>
                <td style={{ padding: 4 }}>{a.call_count}</td>
                <td style={{ padding: 4, fontFamily: "monospace", fontSize: 12 }}>
                  {a.execute_url}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
};

export default ConnectedAgents;
