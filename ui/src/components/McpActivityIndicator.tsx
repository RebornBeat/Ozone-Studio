/**
 * A3 — Live MCP/tool-call activity indicator.
 *
 * Real signal, confirmed by direct source read + a live call against the
 * running host (2026-09-28): every MCP tool invocation IS recorded —
 * `src/mcp.rs`'s `McpRegistry::invoke()` (~line 423) calls
 * `hub.record(ActivityKind::Tool, level, &call.agent, "{agent} called tool
 * {tool} — ok"/"— failed: {err}", None)` on every call, tool registration
 * included. This reaches `ActivityHub` (`src/monitor/mod.rs`), a real
 * 500-event ring buffer, readable via `GET /monitor/activity?kind=tool`
 * (`src/grpc/mod.rs` `list_activity`) — verified live:
 * `curl 'http://127.0.0.1:50051/monitor/activity?kind=tool&limit=5'` →
 * real `{"id","kind":"tool","level","message","source","timestamp"}` rows
 * (5 real "Tool ozone-shared-context registered" events at the time of
 * writing — an actually-invoked tool CALL, not just registration, produces
 * the same shape with message "{agent} called tool {tool} — ok").
 *
 * NOT live-pushed: `mcp_call`'s own response JSON claims "graph ripple
 * emitted for the call" — grepped `src/mcp.rs`, that claim is false, no
 * `graph_events::emit` call exists anywhere in it. There is also no
 * `orchestration_stage`/`pipeline_progress`-style WS broadcast for this hub
 * (`handle_websocket` in `src/grpc/mod.rs` only forwards `graph_events` and
 * `orchestration_events` and polls `executor_progress` — `ActivityHub` isn't
 * wired into any of the three). So this component polls the real REST
 * endpoint instead, same convention as `components/TaskDetailPanel.tsx`'s
 * `setInterval(poll, 2000)` — honest, real, just not push-live.
 */
import React, { useEffect, useRef, useState } from "react";
import { getJson, qs } from "../data/http";

interface ActivityEvent {
  id: number;
  timestamp: number;
  kind: string;
  level: "info" | "ok" | "warn" | "error";
  source: string;
  message: string;
  detail?: unknown;
}

const POLL_MS = 2000;
const LEVEL_COLOR: Record<string, string> = {
  info: "var(--color-text-muted)",
  ok: "#8fe38f",
  warn: "#ffb95f",
  error: "#ff8a8a",
};

/** Parses the real "{agent} called tool {tool} — ok"/"— failed: ..." message
 * shape `invoke()` produces, falling back to the raw message for anything
 * else this hub carries (tool registration, etc.) — never invents a tool
 * name that wasn't really in the message. */
function parseToolCall(message: string): { tool: string; rest: string } | null {
  const m = message.match(/^(.+?) called tool (\S+) — (.+)$/);
  if (!m) return null;
  return { tool: m[2], rest: `${m[1]} · ${m[3]}` };
}

export const McpActivityIndicator: React.FC<{ isRunning: boolean }> = ({ isRunning }) => {
  const [events, setEvents] = useState<ActivityEvent[]>([]);
  const [error, setError] = useState<string | null>(null);
  const seenMaxId = useRef(0);

  useEffect(() => {
    if (!isRunning) {
      setEvents([]);
      seenMaxId.current = 0;
      return;
    }
    let cancelled = false;
    async function poll() {
      try {
        const res = await getJson<{ events: ActivityEvent[] }>(
          `/monitor/activity${qs({ kind: "tool", limit: 20 })}`,
        );
        if (cancelled) return;
        const rows = res?.events ?? [];
        const fresh = rows.filter((e) => e.id > seenMaxId.current);
        if (fresh.length > 0) {
          seenMaxId.current = Math.max(seenMaxId.current, ...rows.map((e) => e.id));
          setEvents((prev) => [...fresh, ...prev].slice(0, 8));
        }
        setError(null);
      } catch (err) {
        if (!cancelled) setError(err instanceof Error ? err.message : String(err));
      }
    }
    // Baseline first (don't show pre-existing history as "just happened"),
    // matching the same convention this session's I3/E-batch forks used.
    getJson<{ events: ActivityEvent[] }>(`/monitor/activity${qs({ kind: "tool", limit: 1 })}`)
      .then((res) => {
        seenMaxId.current = res?.events?.[0]?.id ?? 0;
        if (!cancelled) poll();
      })
      .catch(() => poll());
    const interval = setInterval(poll, POLL_MS);
    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, [isRunning]);

  if (!isRunning || (events.length === 0 && !error)) return null;

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: 3,
        fontSize: 11,
        color: "var(--color-text-muted)",
        padding: "2px 4px",
        maxHeight: 90,
        overflowY: "auto",
      }}
    >
      {error && <span style={{ color: "#ff8a8a" }}>tool activity: {error}</span>}
      {events.map((e) => {
        const parsed = parseToolCall(e.message);
        return (
          <div
            key={e.id}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 6,
              animation: "fadeIn 0.4s ease-out",
            }}
          >
            <span
              style={{
                width: 5,
                height: 5,
                borderRadius: "50%",
                background: LEVEL_COLOR[e.level] ?? "var(--color-text-muted)",
                flex: "none",
              }}
            />
            {parsed ? (
              <span>
                <b style={{ color: "var(--color-text-secondary)" }}>{parsed.tool}</b> — {parsed.rest}
              </span>
            ) : (
              <span>{e.message}</span>
            )}
          </div>
        );
      })}
    </div>
  );
};

export default McpActivityIndicator;
