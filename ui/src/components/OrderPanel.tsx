/**
 * OrderPanel — the Universal Order day view (docs/UNIVERSAL_ORDER_GUIDE.md
 * §4: "the day view (personal assistant)").
 *
 * Reads GET /order/global (a derived view, never a copy — same discipline
 * as MonitoringPanel) and buckets by due_at client-side, mirroring the
 * exact overdue/today/week/upcoming/someday arithmetic the backend itself
 * uses (src/grpc/mod.rs's get_global_order) so a single fetch serves every
 * bucket instead of five round-trips.
 *
 * Quick-capture (Stage 4) creates a real task through the same
 * /task/create path agent-coordination tasks use — a personal item is
 * just a task with no assignee.
 */
import React, { useEffect, useState, useCallback } from "react";
import CreationActivity from "./CreationActivity";
import {
  fetchGlobalOrder,
  createOrderItem,
  fetchAssistantFeed,
  GlobalOrder,
  OrderItem,
  OrderItemKind,
  AssistantFeed,
} from "../ozoneClient";

const KIND_ICONS: Record<OrderItemKind, string> = {
  todo: "✓",
  meeting: "📅",
  followup: "↩",
  note: "📝",
  code: "💻",
  milestone: "🏁",
  external: "🔗",
};

type Bucket = "overdue" | "today" | "week" | "upcoming" | "someday";
const BUCKET_ORDER: Bucket[] = ["overdue", "today", "week", "upcoming", "someday"];
const BUCKET_LABELS: Record<Bucket, string> = {
  overdue: "Overdue",
  today: "Today",
  week: "This Week",
  upcoming: "Upcoming",
  someday: "Someday",
};

function bucketOf(dueAt: number | null): Bucket {
  if (dueAt == null) return "someday";
  const now = Date.now() / 1000;
  const day = 86400;
  if (dueAt < now - day) return "overdue";
  if (dueAt <= now + day) return "today";
  if (dueAt <= now + 7 * day) return "week";
  return "upcoming";
}

function dueLabel(dueAt: number | null): string {
  if (dueAt == null) return "";
  return new Date(dueAt * 1000).toLocaleString([], {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export const OrderPanel: React.FC = () => {
  const [order, setOrder] = useState<GlobalOrder | null>(null);
  const [connected, setConnected] = useState(false);
  const [showActive, setShowActive] = useState(true);
  const [captureText, setCaptureText] = useState("");
  const [captureKind, setCaptureKind] = useState<OrderItemKind>("todo");
  const [captureDue, setCaptureDue] = useState("");
  const [capturing, setCapturing] = useState(false);
  const [captureError, setCaptureError] = useState<string | null>(null);
  // Personal Assistant feed (docs/PERSONAL_ASSISTANT_GUIDE.md §4.2) — the
  // same panel is the day view the guide describes: the feed (what needs
  // attention) sits above the buckets (what's due when). null = fetch
  // failed; [] = genuinely nothing needs attention. Derived read, no LLM.
  const [feed, setFeed] = useState<AssistantFeed | null | undefined>(undefined);

  const load = useCallback(async () => {
    try {
      const result = await fetchGlobalOrder();
      setOrder(result);
      setConnected(true);
    } catch {
      setConnected(false);
    }
    try {
      setFeed(await fetchAssistantFeed());
    } catch {
      setFeed(null);
    }
  }, []);

  useEffect(() => {
    load();
    const interval = setInterval(load, 8000);
    return () => clearInterval(interval);
  }, [load]);

  const handleCapture = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!captureText.trim()) return;
    setCapturing(true);
    setCaptureError(null);
    try {
      const dueAt = captureDue ? Math.floor(new Date(captureDue).getTime() / 1000) : undefined;
      const res = await createOrderItem({
        name: captureText.trim(),
        kind: captureKind,
        dueAt,
        noteBody: captureKind === "note" ? captureText.trim() : undefined,
      });
      if (res && res.success === false) {
        setCaptureError(res.error || "Capture failed");
      } else {
        setCaptureText("");
        setCaptureDue("");
        await load();
      }
    } catch (err) {
      setCaptureError(err instanceof Error ? err.message : "Capture failed");
    } finally {
      setCapturing(false);
    }
  };

  // Active work (live/paused/queued/interrupted) stays separate from the
  // due-date buckets below — a running task may have no due_at at all,
  // and "what's in flight" is a different question from "what's due when".
  const activeItems: OrderItem[] = order
    ? [...order.live, ...order.paused, ...order.queued, ...order.interrupted]
    : [];

  const buckets: Record<Bucket, OrderItem[]> = {
    overdue: [], today: [], week: [], upcoming: [], someday: [],
  };
  if (order) {
    // Meetings/todos/notes not already shown in "active work" above
    // populate the day view, bucketed by due_at. order.other's items
    // (status outside the running/paused/queued/interrupted/done set)
    // flow through here naturally — none of them match the skip check.
    for (const item of [...order.live, ...order.paused, ...order.queued, ...order.interrupted, ...order.other]) {
      if (item.status === "running" || item.status === "paused" || item.status === "queued" || item.status === "interrupted") continue;
      buckets[bucketOf(item.due_at)].push(item);
    }
  }

  return (
    <div className="opanel">
      <div className="opanel-head">
        <span className={`odot ${connected ? "ok" : "err"}`} />
        <span className="opanel-title">Universal Order</span>
      </div>
      <p className="opanel-sub">
        Everything is one order — todos, meetings, notes, and code work, by
        what's due and what's live.
      </p>

      {/* Live creation activity (real /ws graph events): methodologies,
          modality graphs, tasks, coordination findings animate in as the
          consciousness creates them. */}
      <CreationActivity />

      <form onSubmit={handleCapture} style={{ display: "flex", gap: 8, margin: "10px 0 16px", flexWrap: "wrap" }}>
        <select
          value={captureKind}
          onChange={(e) => setCaptureKind(e.target.value as OrderItemKind)}
          style={{ background: "var(--color-surface-2, #1c2430)", color: "#e8eef6", border: "1px solid #2a3444", borderRadius: 6, padding: "6px 8px" }}
        >
          {(Object.keys(KIND_ICONS) as OrderItemKind[]).map((k) => (
            <option key={k} value={k}>{KIND_ICONS[k]} {k}</option>
          ))}
        </select>
        <input
          type="text"
          value={captureText}
          onChange={(e) => setCaptureText(e.target.value)}
          placeholder="Quick capture — a todo, note, or reminder…"
          style={{ flex: 1, minWidth: 200, background: "var(--color-surface-2, #1c2430)", color: "#e8eef6", border: "1px solid #2a3444", borderRadius: 6, padding: "6px 10px" }}
        />
        <input
          type="datetime-local"
          value={captureDue}
          onChange={(e) => setCaptureDue(e.target.value)}
          style={{ background: "var(--color-surface-2, #1c2430)", color: "#e8eef6", border: "1px solid #2a3444", borderRadius: 6, padding: "6px 8px" }}
        />
        <button type="submit" disabled={capturing || !captureText.trim()} className="ochip" style={{ cursor: "pointer" }}>
          {capturing ? "Adding…" : "Add"}
        </button>
      </form>
      {captureError && (
        <div className="oempty" style={{ color: "#e08a8a", marginBottom: 12 }}>{captureError}</div>
      )}

      {feed !== undefined && feed !== null && feed.findings.length > 0 && (
        <div style={{ marginBottom: 18 }}>
          <h4 style={{ margin: "4px 0 8px", color: "#aebdce", fontSize: 12, textTransform: "uppercase", letterSpacing: 0.7 }}>
            Needs attention ({feed.findings.length})
          </h4>
          <div className="ofeed">
            {feed.findings.map((f) => (
              <div key={`${f.class}-${f.task_id}`} className="ofeed-row" style={{ alignItems: "flex-start" }}>
                <span
                  title={f.detail}
                  style={{
                    flexShrink: 0,
                    width: 8, height: 8, borderRadius: "50%", marginTop: 5,
                    background: f.severity >= 3 ? "#e08a8a" : f.severity === 2 ? "#e0c08a" : "#aebdce",
                  }}
                />
                <span style={{ color: "#e8eef6" }}>
                  {f.title}
                  <span style={{ color: "#8a99ab", fontSize: 11, display: "block" }}>{f.detail}</span>
                </span>
                <span className="ofeed-src" style={{ marginLeft: "auto", flexShrink: 0 }}>{f.class.replace(/-/g, " ")}</span>
              </div>
            ))}
          </div>
        </div>
      )}

      <div
        style={{ display: "flex", alignItems: "center", justifyContent: "space-between", margin: "4px 0 8px" }}
      >
        <h4 style={{ margin: 0, color: "#aebdce", fontSize: 12, textTransform: "uppercase", letterSpacing: 0.7 }}>
          Active work ({activeItems.length})
        </h4>
        <button className="oseg" onClick={() => setShowActive((s) => !s)} style={{ cursor: "pointer" }}>
          {showActive ? "hide" : "show"}
        </button>
      </div>
      {showActive && (
        activeItems.length === 0 ? (
          <div className="oempty">Nothing live, paused, queued, or interrupted right now.</div>
        ) : (
          <table className="otable" style={{ marginBottom: 18 }}>
            <thead>
              <tr><th>Kind</th><th>Name</th><th>Status</th><th>Progress</th><th>Due</th></tr>
            </thead>
            <tbody>
              {activeItems.map((it) => (
                <tr key={it.task_id}>
                  <td>{KIND_ICONS[it.kind] ?? "•"}</td>
                  <td style={{ color: "#e8eef6" }}>{it.name}</td>
                  <td><span className={`ochip ${it.status}`}>{it.status}</span></td>
                  <td className="omono">{it.steps_total > 0 ? `${it.steps_done}/${it.steps_total}` : `${Math.round(it.progress * 100)}%`}</td>
                  <td className="omono">{dueLabel(it.due_at)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )
      )}

      {BUCKET_ORDER.map((bucket) => (
        <div key={bucket} style={{ marginBottom: 14 }}>
          <h4 style={{ margin: "0 0 6px", color: bucket === "overdue" ? "#e08a8a" : "#aebdce", fontSize: 12, textTransform: "uppercase", letterSpacing: 0.7 }}>
            {BUCKET_LABELS[bucket]} ({buckets[bucket].length})
          </h4>
          {buckets[bucket].length === 0 ? (
            <div className="oempty" style={{ padding: "4px 0" }}>Nothing here.</div>
          ) : (
            <div className="ofeed">
              {buckets[bucket].map((it) => (
                <div key={it.task_id} className="ofeed-row">
                  <span>{KIND_ICONS[it.kind] ?? "•"}</span>
                  <span className="ofeed-src">{it.name}</span>
                  {it.meeting_url && (
                    <a href={it.meeting_url} target="_blank" rel="noreferrer" style={{ color: "#7aa2f7" }}>join</a>
                  )}
                  <span className="ofeed-time">{dueLabel(it.due_at)}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      ))}
    </div>
  );
};

export default OrderPanel;
