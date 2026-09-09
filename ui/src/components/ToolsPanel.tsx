/**
 * ToolsPanel — the MCP tool registry on the shared design system.
 * Dashboard face of POST /mcp/tools/register (the 90+ tool surface).
 */
import React, { useEffect, useState } from "react";
import { McpToolInfo, fetchMcpTools } from "../ozoneClient";

export const ToolsPanel: React.FC<{ pollMs?: number }> = ({
  pollMs = 8000,
}) => {
  const [tools, setTools] = useState<McpToolInfo[]>([]);
  const [connected, setConnected] = useState(false);

  useEffect(() => {
    let cancelled = false;
    const poll = async () => {
      try {
        const out = await fetchMcpTools();
        if (cancelled) return;
        setTools(out.tools ?? []);
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

  return (
    <div className="opanel">
      <div className="opanel-head">
        <span className={`odot ${connected ? "ok" : "err"}`} />
        <span className="opanel-title">Registered Tools</span>
      </div>
      <p className="opanel-sub">
        MCP tools connect and announce — same pattern as agents. Register with{" "}
        <code style={{ color: "#7fa8cc" }}>POST /mcp/tools/register</code>{" "}
        <span className="omono">
          {'{name, transport: "stdio|http|sse", endpoint}'}
        </span>
        .
      </p>

      <div className="ostats">
        <div className="ostat">
          <div className="ostat-num">{tools.length}</div>
          <div className="ostat-label">Tools</div>
        </div>
      </div>

      {tools.length === 0 ? (
        <div className="oempty">
          No tools registered yet.
          <br />
          The 90+ MCP tool list and the gamedev tooling land here as they
          connect.
        </div>
      ) : (
        <table className="otable">
          <thead>
            <tr>
              <th>Name</th>
              <th>Transport</th>
              <th>Endpoint</th>
              <th>Capabilities</th>
            </tr>
          </thead>
          <tbody>
            {tools.map((t) => (
              <tr key={t.name}>
                <td style={{ color: "#e8eef6", fontWeight: 500 }}>{t.name}</td>
                <td>
                  <span className="ochip cat">{t.transport}</span>
                </td>
                <td className="omono">{t.endpoint}</td>
                <td style={{ opacity: 0.8 }}>
                  {t.capabilities?.join(", ") || "—"}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
};

export default ToolsPanel;
