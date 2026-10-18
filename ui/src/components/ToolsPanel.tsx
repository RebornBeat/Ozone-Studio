/**
 * ToolsPanel — the MCP tool registry on the shared design system.
 * Dashboard face of POST /mcp/tools/register (the 90+ tool surface).
 */
import React, { useEffect, useMemo, useState } from "react";
import {
  McpToolInfo,
  fetchMcpTools,
  setMcpAutoStart,
  startMcpTool,
  stopMcpTool,
} from "../ozoneClient";

/** One row per distinct (transport, endpoint) — the real process-identity
 * unit (src/mcp.rs's McpTool::auto_start doc comment: one Node process can
 * back many registered capability names, e.g. the 67-tool `bridges`
 * fleet on one port). Grouping here mirrors that so toggling one doesn't
 * silently also flip siblings the operator didn't look at. */
function groupByProcess(tools: McpToolInfo[]) {
  const groups = new Map<string, McpToolInfo[]>();
  for (const t of tools) {
    const key = `${t.transport}::${t.endpoint}`;
    const list = groups.get(key) ?? [];
    list.push(t);
    groups.set(key, list);
  }
  return Array.from(groups.entries()).map(([key, members]) => ({ key, members }));
}

const requirementLabel = (device: string | null, credential: string | null) => {
  const parts: string[] = [];
  if (device) parts.push(`needs: ${device} device`);
  if (credential) parts.push(`needs: ${credential} credentials`);
  return parts.join(", ");
};

export const ToolsPanel: React.FC<{ pollMs?: number }> = ({
  pollMs = 8000,
}) => {
  const [tools, setTools] = useState<McpToolInfo[]>([]);
  const [connected, setConnected] = useState(false);
  const [busy, setBusy] = useState<Set<string>>(new Set());
  const [lastError, setLastError] = useState<Record<string, string>>({});

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

  const groups = useMemo(() => groupByProcess(tools), [tools]);

  const withBusy = async (key: string, fn: () => Promise<void>) => {
    setBusy((prev) => new Set(prev).add(key));
    try {
      await fn();
    } finally {
      setBusy((prev) => {
        const next = new Set(prev);
        next.delete(key);
        return next;
      });
    }
  };

  const toggleAutoStart = (lead: McpToolInfo, enabled: boolean) =>
    withBusy(lead.name, async () => {
      const res = await setMcpAutoStart(lead.name, enabled);
      if (!res.success) {
        setLastError((prev) => ({ ...prev, [lead.name]: res.error ?? "toggle failed" }));
        return;
      }
      setLastError((prev) => {
        const { [lead.name]: _drop, ...rest } = prev;
        return rest;
      });
      // Optimistic local update across the whole affected group — the
      // next poll (≤pollMs away) reconciles against the real server state.
      const affected = new Set(res.affected ?? [lead.name]);
      setTools((prev) =>
        prev.map((t) => (affected.has(t.name) ? { ...t, auto_start: enabled } : t)),
      );
    });

  const startNow = (t: McpToolInfo) =>
    withBusy(`start:${t.name}`, async () => {
      const res = await startMcpTool(t.name);
      if (!res.success) {
        setLastError((prev) => ({ ...prev, [t.name]: res.error ?? "start failed" }));
      } else {
        setLastError((prev) => {
          const { [t.name]: _drop, ...rest } = prev;
          return rest;
        });
      }
    });

  const stopNow = (t: McpToolInfo) =>
    withBusy(`stop:${t.name}`, async () => {
      const res = await stopMcpTool(t.name);
      if (!res.success) {
        setLastError((prev) => ({ ...prev, [t.name]: res.error ?? "stop failed" }));
      } else {
        setLastError((prev) => {
          const { [t.name]: _drop, ...rest } = prev;
          return rest;
        });
      }
    });

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
      <p className="opanel-sub">
        <strong>Auto-start is grouped by process, not by tool.</strong> One
        Node process can back many registered capability names — the
        67-tool <code>bridges</code> fleet all run on a single port.
        Toggling Auto-start anywhere in a group flips the whole group;
        rows below are grouped by their shared endpoint so this is never a
        surprise. A tool with a device or credential requirement (e.g.{" "}
        <code>android-mcp</code> needs a real attached Android device)
        won't actually start even with Auto-start on until that
        requirement is met — the supervisor checks honestly every 30s and
        logs the real reason it skipped a tool, rather than starting it
        anyway.
      </p>

      <div className="ostats">
        <div className="ostat">
          <div className="ostat-num">{tools.length}</div>
          <div className="ostat-label">Tools</div>
        </div>
        <div className="ostat">
          <div className="ostat-num">{groups.length}</div>
          <div className="ostat-label">Processes</div>
        </div>
        <div className="ostat">
          <div className="ostat-num">{tools.filter((t) => t.running).length}</div>
          <div className="ostat-label">Running</div>
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
        <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
          {groups.map(({ key, members }) => {
            const lead = members[0];
            const running = members.some((m) => m.running);
            const reqLabel = requirementLabel(lead.device_requirement, lead.credential_requirement);
            const groupBusy = busy.has(lead.name);
            return (
              <div
                key={key}
                style={{
                  border: "1px solid rgba(255,255,255,0.08)",
                  borderRadius: 8,
                  padding: "8px 12px",
                }}
              >
                <div
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: 10,
                    flexWrap: "wrap",
                  }}
                >
                  <span className={`odot ${running ? "ok" : "err"}`} title={running ? "running" : "not running"} />
                  <span className="omono" style={{ color: "#e8eef6", fontWeight: 500 }}>
                    {lead.endpoint}
                  </span>
                  <span className="ochip cat">{lead.transport}</span>
                  {members.length > 1 && (
                    <span className="ochip" title={members.map((m) => m.name).join(", ")}>
                      {members.length} tools on this process
                    </span>
                  )}
                  {reqLabel && (
                    <span className="ochip" style={{ opacity: 0.85 }} title={reqLabel}>
                      {reqLabel}
                    </span>
                  )}
                  <span style={{ flex: 1 }} />
                  <label
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: 6,
                      fontSize: 12,
                      opacity: groupBusy ? 0.5 : 1,
                      cursor: groupBusy ? "default" : "pointer",
                    }}
                  >
                    <input
                      type="checkbox"
                      checked={lead.auto_start}
                      disabled={groupBusy}
                      onChange={(e) => toggleAutoStart(lead, e.target.checked)}
                    />
                    Auto-start
                  </label>
                  <button
                    className="toolbar-btn"
                    disabled={busy.has(`start:${lead.name}`) || running}
                    onClick={() => startNow(lead)}
                    title="Start this process now, bypassing Auto-start"
                  >
                    Start now
                  </button>
                  <button
                    className="toolbar-btn"
                    disabled={busy.has(`stop:${lead.name}`) || !running}
                    onClick={() => stopNow(lead)}
                  >
                    Stop
                  </button>
                </div>
                {lastError[lead.name] && (
                  <div style={{ color: "#ff8a8a", fontSize: 11.5, marginTop: 4 }}>
                    {lastError[lead.name]}
                  </div>
                )}
                <div style={{ marginTop: 6, opacity: 0.8, fontSize: 12 }}>
                  {members.map((m) => m.name).join(", ")}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};

export default ToolsPanel;
