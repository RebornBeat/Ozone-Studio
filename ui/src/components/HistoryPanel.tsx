/**
 * HistoryPanel — the native git-like history of ONE item (src/history; routes /history/*).
 *
 * Shows every recorded version of a workspace file (scope "file") or a ZSEI container record (scope "container"), a line diff between any two
 * versions (computed by the host; binary content shows changed chunks), and an operator-only RESTORE. A restore puts the chosen version back
 * and the host records the CURRENT state first (reason "pre-restore"), so a restore is itself reversible. History is never pruned or capped.
 * Reached from the Files tab (version history of the selected file) and the History tab (every item that has history).
 */
import React, { useCallback, useEffect, useState } from "react";
import { fetchHistoryDiff, fetchHistoryList, restoreHistoryVersion, type HistoryDiff, type HistoryRow } from "../ozoneClient";

const box: React.CSSProperties = { border: "1px solid var(--color-border-faint, #2a3a4d)", borderRadius: 6, padding: "6px 8px" };
const btn: React.CSSProperties = { padding: "2px 9px", borderRadius: 6, border: "1px solid #3a5470", background: "#162434", color: "#d7e4f3", cursor: "pointer", fontSize: 12 };
const muted: React.CSSProperties = { color: "var(--color-text-muted, #8fa7bf)", fontSize: 11.5 };

function when(ts: number): string {
  const ms = ts > 1e12 ? ts : ts * 1000;
  const d = new Date(ms);
  return Number.isNaN(d.getTime()) ? String(ts) : d.toLocaleString();
}

export interface HistoryPanelProps {
  scope: "file" | "container" | string;
  id: string | number;
  title?: string;
  /** called after a successful restore so the parent can refresh what it shows */
  onRestored?: () => void;
  pollMs?: number;
}

export const HistoryPanel: React.FC<HistoryPanelProps> = ({ scope, id, title, onRestored, pollMs = 15000 }) => {
  const [rows, setRows] = useState<HistoryRow[] | null>(null);
  const [err, setErr] = useState("");
  const [from, setFrom] = useState<number | null>(null);
  const [to, setTo] = useState<number | null>(null);
  const [diff, setDiff] = useState<HistoryDiff | null>(null);
  const [diffErr, setDiffErr] = useState("");
  const [confirmFor, setConfirmFor] = useState<number | null>(null);
  const [typed, setTyped] = useState("");
  const [msg, setMsg] = useState("");

  const load = useCallback(async () => {
    try {
      const r = await fetchHistoryList(scope, id);
      if (r.success) {
        setRows([...r.versions].sort((a, b) => b.v - a.v));
        setErr("");
      } else {
        setErr(r.error ?? "history unavailable");
      }
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : String(e));
    }
  }, [scope, id]);

  useEffect(() => {
    setRows(null);
    setDiff(null);
    setFrom(null);
    setTo(null);
    void load();
    const t = window.setInterval(() => void load(), pollMs);
    return () => window.clearInterval(t);
  }, [load, pollMs]);

  const showDiff = async (f: number | null, t: number | null) => {
    setDiffErr("");
    try {
      const r = await fetchHistoryDiff(scope, id, f ?? undefined, t ?? undefined);
      if (r.success) setDiff(r.diff);
      else {
        setDiff(null);
        setDiffErr(r.error ?? "diff unavailable");
      }
    } catch (e: unknown) {
      setDiffErr(e instanceof Error ? e.message : String(e));
    }
  };

  const restore = async (v: number) => {
    const r = await restoreHistoryVersion(scope, id, v);
    setMsg(r.success ? `Restored: ${r.restored ?? `version ${v}`}. The previous state was recorded first.` : `Not restored: ${r.error ?? "refused"}`);
    setConfirmFor(null);
    setTyped("");
    void load();
    if (r.success) onRestored?.();
  };

  if (err && !rows) return <div className="oempty" style={{ textAlign: "left" }}>History unavailable: {err}</div>;
  if (!rows) return <div className="oempty" style={{ textAlign: "left" }}>Loading history…</div>;
  if (rows.length === 0) {
    return (
      <div className="oempty" style={{ textAlign: "left" }}>
        No versions recorded yet for {scope} {id}. A file is recorded the first time the beacon sees it and on every change; a container is recorded
        just before each update or delete.
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
      {title && <div className="omono" style={{ fontWeight: 600 }}>{title}</div>}
      <div style={muted}>
        {rows.length} version{rows.length === 1 ? "" : "s"} · never pruned · content is stored once per distinct chunk, so history is much smaller than full copies
      </div>
      {msg && <div style={{ ...box, color: "#9fd4b4" }}>{msg}</div>}
      <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
        {rows.map((r, i) => (
          <div key={r.v} style={box}>
            <div style={{ display: "flex", justifyContent: "space-between", gap: 8, flexWrap: "wrap" }}>
              <span>
                <strong>v{r.v}</strong> · {r.reason}
                {i === 0 ? <span style={{ ...muted, marginLeft: 6 }}>(newest recorded)</span> : null}
              </span>
              <span style={muted}>{when(r.ts)} · {r.size} bytes</span>
            </div>
            {r.path ? <div style={muted}>{r.path}</div> : null}
            <div style={{ display: "flex", gap: 6, marginTop: 4, flexWrap: "wrap" }}>
              <button style={btn} onClick={() => { const prev = rows[i + 1]?.v ?? null; setFrom(prev); setTo(r.v); void showDiff(prev, r.v); }} disabled={i === rows.length - 1}>
                Changes in this version
              </button>
              <button style={btn} onClick={() => { setTo(r.v); if (from !== null) void showDiff(from, r.v); }}>Compare to… (to)</button>
              <button style={btn} onClick={() => { setFrom(r.v); if (to !== null) void showDiff(r.v, to); }}>Compare from…</button>
              {confirmFor === r.v ? (
                <span style={{ display: "inline-flex", gap: 6, alignItems: "center" }}>
                  <input
                    value={typed}
                    onChange={(e) => setTyped(e.target.value)}
                    placeholder="type RESTORE"
                    style={{ background: "#0c141c", border: "1px solid #2a3a4d", borderRadius: 6, color: "#d7e4f3", padding: "2px 6px", fontSize: 12, width: 110 }}
                  />
                  <button style={btn} disabled={typed !== "RESTORE"} onClick={() => void restore(r.v)}>Confirm restore</button>
                  <button style={btn} onClick={() => { setConfirmFor(null); setTyped(""); }}>Cancel</button>
                </span>
              ) : (
                <button style={btn} onClick={() => { setConfirmFor(r.v); setTyped(""); }}>Restore this version…</button>
              )}
            </div>
          </div>
        ))}
      </div>
      {(from !== null || to !== null) && (
        <div style={muted}>Comparing {from !== null ? `v${from}` : "(previous)"} → {to !== null ? `v${to}` : "(newest)"}</div>
      )}
      {diffErr && <div className="oempty" style={{ textAlign: "left" }}>Diff unavailable: {diffErr}</div>}
      {diff && (
        <div style={box}>
          {diff.note && <div style={muted}>{diff.note}</div>}
          {diff.binary ? (
            <div>
              Binary content: {diff.chunks_added} chunk(s) added, {diff.chunks_removed} removed, {diff.chunks_shared} shared (of {diff.chunks_to}).
            </div>
          ) : (
            <>
              {diff.coarse && <div style={{ ...muted, color: "#f0b27a" }}>Very large difference: shown as the whole differing region replaced.</div>}
              {(diff.hunks ?? []).length === 0 && !diff.note && <div style={muted}>No line differences.</div>}
              {(diff.hunks ?? []).map((h, hi) => (
                <pre key={hi} style={{ margin: "6px 0", fontSize: 12, overflowX: "auto", background: "#0c141c", padding: 6, borderRadius: 6 }}>
                  <div style={{ color: "#7f94ab" }}>{h.header}</div>
                  {h.lines.map((l, li) => (
                    <div key={li} style={{ color: l.startsWith("+") ? "#7fd1a8" : l.startsWith("-") ? "#f08a8a" : "#c9d6e6" }}>{l || " "}</div>
                  ))}
                </pre>
              ))}
            </>
          )}
        </div>
      )}
    </div>
  );
};

export default HistoryPanel;
