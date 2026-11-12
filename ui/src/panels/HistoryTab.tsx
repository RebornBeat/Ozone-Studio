/**
 * HistoryTab — every item the host keeps a version history of (GET-style POST /history/status): workspace files the beacon watches, evidence/
 * checklist/step documents under the host's watch roots, and ZSEI container records (snapshotted before each update/delete). Pick one to see
 * its versions, diffs and the operator-only restore (HistoryPanel).
 */
import React, { useEffect, useMemo, useState } from "react";
import HistoryPanel from "../components/HistoryPanel";
import { fetchHistoryStatus, type HistoryItem } from "../ozoneClient";

function when(ts: number): string {
  const ms = ts > 1e12 ? ts : ts * 1000;
  const d = new Date(ms);
  return Number.isNaN(d.getTime()) ? String(ts) : d.toLocaleString();
}

export const HistoryTab: React.FC<{ pollMs?: number }> = ({ pollMs = 20000 }) => {
  const [items, setItems] = useState<HistoryItem[] | null>(null);
  const [err, setErr] = useState("");
  const [scope, setScope] = useState<"all" | "file" | "container">("all");
  const [q, setQ] = useState("");
  const [sel, setSel] = useState<HistoryItem | null>(null);

  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      try {
        const r = await fetchHistoryStatus(scope === "all" ? undefined : scope);
        if (cancelled) return;
        if (r.success) {
          setItems(r.items);
          setErr("");
        } else setErr(r.error ?? "history unavailable");
      } catch (e: unknown) {
        if (!cancelled) setErr(e instanceof Error ? e.message : String(e));
      }
    };
    void load();
    const t = window.setInterval(() => void load(), pollMs);
    return () => {
      cancelled = true;
      window.clearInterval(t);
    };
  }, [scope, pollMs]);

  const shown = useMemo(() => {
    const needle = q.trim().toLowerCase();
    return (items ?? []).filter((i) => !needle || i.id.toLowerCase().includes(needle) || (i.path ?? "").toLowerCase().includes(needle));
  }, [items, q]);

  return (
    <div style={{ display: "flex", gap: 12, padding: 12, height: "100%", boxSizing: "border-box" }}>
      <div style={{ width: 380, display: "flex", flexDirection: "column", gap: 8, minWidth: 0 }}>
        <div className="omono" style={{ fontWeight: 600 }}>History</div>
        <div className="oempty" style={{ textAlign: "left" }}>
          Every change to a watched file or container is kept (never pruned). Files under the host's watch roots (checklists, step and evidence documents)
          appear here too.
        </div>
        <div style={{ display: "flex", gap: 6 }}>
          <select value={scope} onChange={(e) => setScope(e.target.value as "all" | "file" | "container")}>
            <option value="all">Files and containers</option>
            <option value="file">Files</option>
            <option value="container">Containers</option>
          </select>
          <input value={q} onChange={(e) => setQ(e.target.value)} placeholder="filter by path or id" style={{ flex: 1 }} />
        </div>
        {err && <div className="oempty" style={{ textAlign: "left" }}>History unavailable: {err}</div>}
        {!items && !err && <div className="oempty">Loading…</div>}
        <div style={{ overflowY: "auto", display: "flex", flexDirection: "column", gap: 4 }}>
          {shown.map((i) => (
            <button
              key={`${i.scope}/${i.id}`}
              onClick={() => setSel(i)}
              style={{
                textAlign: "left",
                padding: "6px 8px",
                borderRadius: 6,
                border: sel && sel.scope === i.scope && sel.id === i.id ? "1px solid #5aa0e0" : "1px solid #2a3a4d",
                background: "#101923",
                color: "#d7e4f3",
                cursor: "pointer",
              }}
            >
              <div style={{ fontSize: 12.5, overflowWrap: "anywhere" }}>{i.path ?? `${i.scope} ${i.id}`}</div>
              <div style={{ fontSize: 11, color: "#8fa7bf" }}>
                {i.scope} {i.id} · {i.versions} version{i.versions === 1 ? "" : "s"} · last {when(i.last_ts)} ({i.last_reason})
              </div>
            </button>
          ))}
          {items && shown.length === 0 && <div className="oempty">Nothing recorded yet{q ? " for that filter" : ""}.</div>}
        </div>
      </div>
      <div style={{ flex: 1, minWidth: 0, overflowY: "auto" }}>
        {sel ? (
          <HistoryPanel scope={sel.scope} id={sel.id} title={sel.path ?? `${sel.scope} ${sel.id}`} />
        ) : (
          <div className="oempty" style={{ textAlign: "left" }}>Pick an item on the left to see its versions, changes and restore.</div>
        )}
      </div>
    </div>
  );
};

export default HistoryTab;
