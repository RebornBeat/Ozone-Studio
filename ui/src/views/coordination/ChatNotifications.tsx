/**
 * I3 — real coordination/capture-store notifications, shown as a compact
 * strip appended to the chat transcript (mounted by MetaPortion.tsx).
 *
 * Real sources only, polled on the same 2s cadence TaskDetailPanel.tsx uses
 * for its own live polling:
 *   - GET /coordination/claims (data/coordinationData.fetchClaims) — real,
 *     landed (B11). The response is CURRENT STATE ONLY (file -> holder), no
 *     event log, so "claimed"/"released" are INFERRED by diffing consecutive
 *     polls: a file appearing = claimed, a file disappearing = released.
 *     Labelled as inferred, never presented as a first-class event type the
 *     backend actually emits.
 *   - GET /capture/zero-shot-calls, GET /capture/decision-reviews
 *     (data/captureData) — real, landed (B4/B5). Both are flat append-only
 *     JSONL files read in file order (oldest -> newest) with no `since`/sort
 *     param and no total count. "New since last poll" is inferred the same
 *     way: fetch a capped window from offset 0 each poll, and treat any rows
 *     beyond the previous poll's count as new (valid because the store is
 *     strictly append-only). NOTIFICATION_FETCH_LIMIT caps how far back a
 *     single poll can see; if the store grows past that between polls some
 *     new rows could be missed — acceptable for a live "recent activity"
 *     strip, not something this component tries to guarantee completeness on.
 *
 * Explicitly NOT built: "Fork Y started/completed Z" notifications. Fork
 * dispatch/task-notification status is a client-side-only mechanism (this
 * chat session's own background-task events) with no durable backend record
 * this component can read — fabricating it here would violate the no-mock
 * doctrine. A single static caption says so once; I4 (fork dispatch
 * visualization) may have access to something this component doesn't and is
 * a separate file/decision, not assumed here.
 *
 * The FIRST poll of each source only establishes a baseline (claims held /
 * rows already on disk) and never emits notifications for it — otherwise
 * every pre-existing claim and every historical row would falsely appear as
 * "just happened" the moment this component mounts.
 */
import React, { useEffect, useRef, useState } from "react";
import { fetchClaims, FileClaim } from "../../data/coordinationData";
import { fetchZeroShotCalls, fetchDecisionReviews, ZeroShotCallRow, DecisionReviewRow } from "../../data/captureData";
import { classifyZeroShotCall, classifyDecisionReview, SEVERITY_STYLE } from "../capture/captureStatus";

const POLL_MS = 2000;
const NOTIFICATION_FETCH_LIMIT = 500;
const MAX_RETAINED = 30;
const MAX_SHOWN = 8;

type Kind = "claim" | "release" | "zero-shot" | "decision-review";

interface NotificationItem {
  id: string;
  kind: Kind;
  at: number; // Date.now() when THIS component observed it (real observation time, not a fabricated event timestamp for claims, which carry none).
  text: string;
  color: string;
  glyph: string;
  title?: string;
}

function short(path: string, n = 46): string {
  return path.length > n ? `…${path.slice(-(n - 1))}` : path;
}

export const ChatNotificationStream: React.FC = () => {
  const [items, setItems] = useState<NotificationItem[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [collapsed, setCollapsed] = useState(false);

  const prevClaims = useRef<Map<string, FileClaim> | null>(null);
  const prevZeroShotCount = useRef<number | null>(null);
  const prevDecisionCount = useRef<number | null>(null);
  const seq = useRef(0);
  const cancelled = useRef(false);

  function push(newOnes: Omit<NotificationItem, "id">[]) {
    if (newOnes.length === 0) return;
    setItems((prev) => {
      const withIds = newOnes.map((n) => ({ ...n, id: `n${seq.current++}` }));
      // Newest first; cap retained history so this strip can't grow unbounded
      // over a long-running chat session.
      return [...withIds.reverse(), ...prev].slice(0, MAX_RETAINED);
    });
  }

  async function pollClaims() {
    const { claims } = await fetchClaims();
    const current = new Map(claims.map((c) => [c.file, c]));
    const prior = prevClaims.current;
    prevClaims.current = current;
    if (!prior) return; // baseline only

    const notes: Omit<NotificationItem, "id">[] = [];
    for (const [file, c] of current) {
      if (!prior.has(file)) {
        notes.push({
          kind: "claim",
          at: Date.now(),
          text: `${c.agent ?? "unknown agent"} claimed ${short(file)}`,
          color: "#8fa8d8",
          glyph: "🔒",
          title: c.reason || undefined,
        });
      }
    }
    for (const [file, c] of prior) {
      if (!current.has(file)) {
        notes.push({
          kind: "release",
          at: Date.now(),
          // Inferred, not a real "release" event the backend emits — say so.
          text: `${c.agent ?? "unknown agent"} released ${short(file)} (inferred: claim no longer listed)`,
          color: "#8b98ab",
          glyph: "🔓",
        });
      }
    }
    push(notes);
  }

  async function pollZeroShot() {
    const { rows } = await fetchZeroShotCalls({ limit: NOTIFICATION_FETCH_LIMIT });
    const prior = prevZeroShotCount.current;
    prevZeroShotCount.current = rows.length;
    if (prior === null) return; // baseline only
    if (rows.length <= prior) return; // append-only store: no growth = nothing new (a shrink would mean a rotated file, not handled specially)

    const fresh: ZeroShotCallRow[] = rows.slice(prior);
    push(
      fresh.map((row) => {
        const status = classifyZeroShotCall(row);
        const style = SEVERITY_STYLE[status.severity];
        const model = row.model_used || "no model";
        return {
          kind: "zero-shot" as const,
          at: Date.now(),
          text: `${row.call_site}: ${model} — ${status.label}`,
          color: style.color,
          glyph: style.glyph,
          title: status.reasons.join("\n"),
        };
      }),
    );
  }

  async function pollDecisionReviews() {
    const { rows } = await fetchDecisionReviews({ limit: NOTIFICATION_FETCH_LIMIT });
    const prior = prevDecisionCount.current;
    prevDecisionCount.current = rows.length;
    if (prior === null) return;
    if (rows.length <= prior) return;

    const fresh: DecisionReviewRow[] = rows.slice(prior);
    push(
      fresh.map((row) => {
        const status = classifyDecisionReview(row);
        const style = SEVERITY_STYLE[status.severity];
        return {
          kind: "decision-review" as const,
          at: Date.now(),
          text: `review: ${status.label}${row.model_used ? ` (${row.model_used})` : ""}`,
          color: style.color,
          glyph: style.glyph,
          title: status.reasons.join("\n"),
        };
      }),
    );
  }

  useEffect(() => {
    cancelled.current = false;
    const tick = async () => {
      try {
        await Promise.all([pollClaims(), pollZeroShot(), pollDecisionReviews()]);
        if (!cancelled.current) setError(null);
      } catch (err) {
        if (!cancelled.current) setError(err instanceof Error ? err.message : String(err));
      }
    };
    tick();
    const id = setInterval(tick, POLL_MS);
    return () => {
      cancelled.current = true;
      clearInterval(id);
    };
  }, []);

  if (items.length === 0 && !error) {
    return null; // nothing real to show yet — no placeholder banner
  }

  const shown = collapsed ? [] : items.slice(0, MAX_SHOWN);

  return (
    <div
      style={{
        marginTop: 8,
        paddingTop: 8,
        borderTop: "1px solid #1e2836",
        fontSize: 11.5,
        color: "#8b98ab",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: collapsed ? 0 : 4 }}>
        <span style={{ fontWeight: 700, color: "#dfe7f2" }}>Coordination activity</span>
        <span style={{ opacity: 0.7 }}>
          (real file claims + capture-store rows, polled live — no fork-dispatch status available yet)
        </span>
        <button
          onClick={() => setCollapsed((c) => !c)}
          style={{
            marginLeft: "auto",
            background: "none",
            border: "1px solid #223046",
            borderRadius: 999,
            color: "inherit",
            fontSize: 11,
            padding: "1px 8px",
            cursor: "pointer",
          }}
        >
          {collapsed ? `show (${items.length})` : "hide"}
        </button>
      </div>
      {error && <div style={{ color: "#ff8a8a" }}>Live updates paused: {error}</div>}
      {!collapsed &&
        shown.map((n) => (
          <div
            key={n.id}
            title={n.title}
            style={{
              display: "flex",
              gap: 6,
              alignItems: "baseline",
              color: n.color,
              lineHeight: 1.6,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            <span aria-hidden="true">{n.glyph}</span>
            <span style={{ overflow: "hidden", textOverflow: "ellipsis" }}>{n.text}</span>
          </div>
        ))}
      {!collapsed && items.length > MAX_SHOWN && (
        <div style={{ opacity: 0.6 }}>+{items.length - MAX_SHOWN} earlier this session</div>
      )}
    </div>
  );
};

export default ChatNotificationStream;
