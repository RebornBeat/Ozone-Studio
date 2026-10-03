/**
 * S13 tool-call capture panel — every real /mcp/call, exactly as captured
 * (tool_calls.jsonl). Same shape/pagination convention as DecisionReviewPanel
 * (S10) / RawThoughtPanel (S11): GET /capture/tool-calls, no `total` in the
 * response, page until a short page comes back, no live push (flat
 * append-only file, use Reload).
 *
 * Built ahead of the backend route landing (tracked in
 * docs/guides/coordination-ripple-gaps.md) — shows a real, honest "could not
 * load" state until it exists rather than hiding the feature; the wire-up
 * needs no further UI change once the route is live.
 *
 * `identity_validated` (src/grpc/mod.rs's mcp_call handler) is real,
 * per-call provenance — whether the caller's session_token was actually
 * checked against AuthSystem for that specific call, not a blanket flag.
 */
import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { fetchToolCalls, ToolCallRow } from "../../data/captureData";
import { CaptureStatusBadge, classifyToolCall } from "./captureStatus";

export type ToolCallPanelProps = { projectId: number | null };

const PAGE = 100;

const C_TEXT = "var(--color-text)";
const C_BODY = "var(--color-text-secondary)";
const C_MUTED = "var(--color-text-muted)";
const C_BORDER = "var(--color-border-faint)";
const C_PANEL = "var(--color-bg)";
const C_OK = "#8fe38f";
const C_WARN = "#e8c14f";

const selectStyle: React.CSSProperties = {
  background: "#101724",
  color: C_TEXT,
  border: `1px solid ${C_BORDER}`,
  borderRadius: 6,
  padding: "3px 8px",
  fontSize: 12,
  maxWidth: 260,
};
const preStyle: React.CSSProperties = {
  margin: "4px 0 0",
  background: C_PANEL,
  border: `1px solid ${C_BORDER}`,
  borderRadius: 6,
  padding: "6px 8px",
  fontSize: 11.5,
  color: C_BODY,
  whiteSpace: "pre-wrap",
  overflowWrap: "anywhere",
  maxHeight: 160,
  overflowY: "auto",
  fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
};

const IdentityBadge: React.FC<{ validated: boolean }> = ({ validated }) => (
  <span
    title={
      validated
        ? "session_token was checked against AuthSystem for this call and verified."
        : "No session_token was verified for this call — the agent name is unauthenticated."
    }
    style={{
      display: "inline-flex",
      alignItems: "center",
      gap: 4,
      border: `1px solid ${validated ? C_OK : C_WARN}`,
      color: validated ? C_OK : C_WARN,
      borderRadius: 999,
      padding: "0 7px",
      fontSize: 11,
      cursor: "help",
    }}
  >
    {validated ? "🔒 identity verified" : "🔓 unverified"}
  </span>
);

export const ToolCallPanel: React.FC<ToolCallPanelProps> = ({ projectId }) => {
  const [rows, setRows] = useState<ToolCallRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [hasMore, setHasMore] = useState(false);
  const [toolFilter, setToolFilter] = useState("all");
  const [agentFilter, setAgentFilter] = useState("all");
  const [newestFirst, setNewestFirst] = useState(true);
  const [expanded, setExpanded] = useState<Set<number>>(new Set());
  const nextOffset = useRef(0);
  const inFlight = useRef(false);

  const loadPage = useCallback(async (reset: boolean) => {
    if (inFlight.current) return;
    inFlight.current = true;
    if (reset) nextOffset.current = 0;
    setLoading(true);
    setError(null);
    try {
      const page = await fetchToolCalls({ offset: nextOffset.current, limit: PAGE });
      nextOffset.current += page.rows.length;
      setRows((prev) => (reset ? page.rows : [...prev, ...page.rows]));
      setHasMore(page.rows.length >= PAGE);
      if (reset) setExpanded(new Set());
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      inFlight.current = false;
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadPage(true);
  }, [loadPage]);

  const toolOptions = useMemo(() => Array.from(new Set(rows.map((r) => r.tool))).sort(), [rows]);
  const agentOptions = useMemo(() => Array.from(new Set(rows.map((r) => r.agent))).sort(), [rows]);
  const failedCount = useMemo(() => rows.filter((r) => !r.success).length, [rows]);
  const unverifiedCount = useMemo(() => rows.filter((r) => !r.identity_validated).length, [rows]);

  const visible = useMemo(() => {
    const filtered = rows.filter(
      (r) => (toolFilter === "all" || r.tool === toolFilter) && (agentFilter === "all" || r.agent === agentFilter),
    );
    return newestFirst ? [...filtered].reverse() : filtered;
  }, [rows, toolFilter, agentFilter, newestFirst]);

  const toggle = (i: number) =>
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(i)) next.delete(i);
      else next.add(i);
      return next;
    });

  return (
    <div style={{ color: C_BODY, fontSize: 12.5 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 10, flexWrap: "wrap", marginBottom: 6 }}>
        <span style={{ fontSize: 13, fontWeight: 700, color: C_TEXT }}>Tool calls</span>
        <button
          onClick={() => loadPage(true)}
          disabled={loading}
          style={{ ...selectStyle, cursor: loading ? "default" : "pointer" }}
          title="Re-read the capture file — this store emits no live events"
        >
          {loading ? "Loading…" : "Reload"}
        </button>
      </div>
      <div style={{ fontSize: 11.5, color: C_MUTED, marginBottom: 10, lineHeight: 1.5 }}>
        Every real /mcp/call exactly as captured (tool_calls.jsonl), including whether the caller's identity was
        actually verified for that specific call.
        {projectId !== null &&
          ` Tool calls are not stored per project, so all of them are shown (project ${projectId} cannot filter this list).`}
      </div>

      {error && (
        <div style={{ border: "1px solid #ff8a8a", color: "#ff8a8a", borderRadius: 6, padding: "6px 8px", marginBottom: 10 }}>
          Could not load tool calls: {error}
          {(error.toLowerCase().includes("not live yet") || error.toLowerCase().includes("404")) && (
            <div style={{ marginTop: 4, fontWeight: 400 }}>
              This is expected until the GET /capture/tool-calls route lands on the host — the write side
              (tool_calls.jsonl) is already real and live, this view just can't read it back yet.
            </div>
          )}
        </div>
      )}

      {!error && !loading && rows.length === 0 && (
        <div style={{ color: C_MUTED, padding: "12px 0" }}>
          No tool calls have been captured yet — nothing to show (nothing is fabricated in its place).
        </div>
      )}
      {!error && loading && rows.length === 0 && <div style={{ color: C_MUTED, padding: "12px 0" }}>Loading tool calls…</div>}

      {rows.length > 0 && (
        <>
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center", marginBottom: 10 }}>
            <span style={{ color: failedCount > 0 ? "#ff8a8a" : C_MUTED, fontWeight: 600 }}>
              {failedCount} of {rows.length} loaded call{rows.length === 1 ? "" : "s"} failed
            </span>
            <span style={{ color: unverifiedCount > 0 ? C_WARN : C_MUTED, fontWeight: 600 }}>
              {unverifiedCount} of {rows.length} unverified identity
            </span>
          </div>

          <div style={{ display: "flex", gap: 10, flexWrap: "wrap", alignItems: "center", marginBottom: 10 }}>
            <label style={{ color: C_MUTED }}>
              Tool{" "}
              <select value={toolFilter} onChange={(e) => setToolFilter(e.target.value)} style={selectStyle}>
                <option value="all">all</option>
                {toolOptions.map((t) => (
                  <option key={t} value={t}>
                    {t}
                  </option>
                ))}
              </select>
            </label>
            <label style={{ color: C_MUTED }}>
              Agent{" "}
              <select value={agentFilter} onChange={(e) => setAgentFilter(e.target.value)} style={selectStyle}>
                <option value="all">all</option>
                {agentOptions.map((a) => (
                  <option key={a} value={a}>
                    {a}
                  </option>
                ))}
              </select>
            </label>
            <label style={{ color: C_MUTED, display: "flex", alignItems: "center", gap: 4, cursor: "pointer" }}>
              <input type="checkbox" checked={newestFirst} onChange={() => setNewestFirst((v) => !v)} style={{ margin: 0 }} />
              Newest first
            </label>
            <span style={{ color: C_MUTED, fontSize: 11.5 }}>
              showing {visible.length} of {rows.length}
            </span>
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            {visible.map((row, i) => {
              const index = rows.indexOf(row);
              const open = expanded.has(index);
              const status = classifyToolCall(row);
              return (
                <div
                  key={`${row.ts}-${i}`}
                  style={{
                    border: `1px solid ${C_BORDER}`,
                    borderLeft: `3px solid ${row.success ? C_OK : "#ff8a8a"}`,
                    borderRadius: 8,
                    padding: "7px 10px",
                    background: "rgba(255,255,255,0.015)",
                  }}
                >
                  <div
                    onClick={() => toggle(index)}
                    style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap", cursor: "pointer" }}
                  >
                    <span style={{ color: C_MUTED }}>{open ? "▾" : "▸"}</span>
                    <span style={{ color: C_MUTED, fontSize: 11.5 }}>{row.ts}</span>
                    <span style={{ color: C_TEXT, fontWeight: 600 }}>{row.tool}</span>
                    <span style={{ color: C_MUTED, fontSize: 11.5 }}>by {row.agent}</span>
                    <CaptureStatusBadge status={status} />
                    <IdentityBadge validated={row.identity_validated} />
                  </div>
                  {open && (
                    <div style={{ marginLeft: 18 }}>
                      <div style={{ marginTop: 6, fontSize: 12, color: C_BODY }}>Transport: {row.transport}</div>
                      <div style={{ marginTop: 2 }}>
                        <div style={{ fontSize: 11, fontWeight: 700, letterSpacing: 0.4, textTransform: "uppercase", color: C_MUTED }}>
                          Input preview
                        </div>
                        <pre style={preStyle}>{row.input_preview || "(empty)"}</pre>
                      </div>
                      {!row.success && (
                        <div style={{ marginTop: 8 }}>
                          <div style={{ fontSize: 11, fontWeight: 700, letterSpacing: 0.4, textTransform: "uppercase", color: C_MUTED }}>
                            Error
                          </div>
                          <pre style={preStyle}>{row.error || "(no error text recorded)"}</pre>
                        </div>
                      )}
                    </div>
                  )}
                </div>
              );
            })}
            {visible.length === 0 && <div style={{ color: C_MUTED, padding: "8px 0" }}>No loaded rows match these filters.</div>}
          </div>

          <div style={{ marginTop: 12, display: "flex", gap: 10, alignItems: "center", flexWrap: "wrap" }}>
            {hasMore && (
              <button onClick={() => loadPage(false)} disabled={loading} style={{ ...selectStyle, cursor: loading ? "default" : "pointer" }}>
                {loading ? "Loading…" : `Load ${PAGE} more`}
              </button>
            )}
            <span style={{ color: C_MUTED, fontSize: 11.5 }}>
              {hasMore
                ? "The file is stored oldest-first, so newer rows arrive as you load more."
                : "End of the capture file — every stored row is loaded."}
            </span>
          </div>
        </>
      )}
    </div>
  );
};

export default ToolCallPanel;
