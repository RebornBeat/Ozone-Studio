/**
 * ConnectedAgents — live view of pipelines registered with the host
 * (the connect-model registry: GET /pipelines/remote via the preload bridge).
 *
 * Any pipeline that booted with --serve and announced itself appears here:
 * id, name, execute URL, registration time, and live call count. This is the
 * dashboard surface of src/pipeline/remote.rs.
 */
import React, { useEffect, useState } from "react";

interface RemotePipeline {
  pipeline_id: number;
  name: string;
  execute_url: string;
  registered_at: number;
  call_count: number;
}

interface Props {
  /** Poll interval in ms (default 5s). */
  pollMs?: number;
  /** Compact mode for embedding in a status bar. */
  compact?: boolean;
}

export const ConnectedAgents: React.FC<Props> = ({
  pollMs = 5000,
  compact = false,
}) => {
  const [agents, setAgents] = useState<RemotePipeline[]>([]);
  const [connected, setConnected] = useState(false);
  const [lastUpdate, setLastUpdate] = useState<Date | null>(null);

  useEffect(() => {
    let cancelled = false;

    const poll = async () => {
      try {
        const oz = (window as any).ozone;
        if (!oz?.pipelinesRemote) {
          setConnected(false);
          return;
        }
        const result = await oz.pipelinesRemote();
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
              <th style={{ padding: 4 }}>Execute URL</th>
              <th style={{ padding: 4 }}>Calls</th>
              <th style={{ padding: 4 }}>Since</th>
            </tr>
          </thead>
          <tbody>
            {agents.map((a) => (
              <tr key={a.pipeline_id}>
                <td style={{ padding: 4 }}>{a.pipeline_id}</td>
                <td style={{ padding: 4 }}>{a.name}</td>
                <td style={{ padding: 4, fontFamily: "monospace", fontSize: 12 }}>
                  {a.execute_url}
                </td>
                <td style={{ padding: 4 }}>{a.call_count}</td>
                <td style={{ padding: 4 }}>
                  {new Date(a.registered_at * 1000).toLocaleTimeString()}
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
