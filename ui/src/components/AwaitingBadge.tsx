/**
 * AwaitingBadge — one honest chip for a task that is AWAITING a judgment.
 * resolver "meta": consciousness/meta re-judges it itself (retrying, no human).
 * resolver "human": jurisdiction confirmation only the operator resolves.
 * Renders only fields the host actually supplied; nothing is invented.
 */
import React from "react";
import { TaskStateIcon } from "../graphkit/indicators";
import type { AwaitingOrderItem } from "../ozoneClient";

export function formatTs(ts?: number | null): string | null {
  if (!ts) return null;
  const ms = ts < 1e12 ? ts * 1000 : ts;
  const d = new Date(ms);
  return isNaN(d.getTime()) ? null : d.toLocaleTimeString();
}

export const AwaitingBadge: React.FC<{ item: Partial<AwaitingOrderItem>; compact?: boolean }> = ({ item, compact }) => {
  const human = item.resolver === "human";
  const label = human ? "awaiting you" : "awaiting meta";
  const next = formatTs(item.next_try_ts);
  const parts = [
    item.kind?.replace(/_/g, " "),
    item.attempts != null ? `${item.attempts} attempt${item.attempts === 1 ? "" : "s"}` : null,
    !human && next ? `next try ${next}` : null,
  ].filter(Boolean);
  return (
    <span
      title={item.last_error ? `Last error: ${item.last_error}` : label}
      style={{ display: "inline-flex", alignItems: "center", gap: 4, fontSize: 10, color: "#d9b45f" }}
    >
      <TaskStateIcon status={human ? "awaiting_human" : "awaiting_meta"} size={12} />
      <span>{label}</span>
      {!compact && parts.length > 0 && <span style={{ opacity: 0.75 }}>· {parts.join(" · ")}</span>}
    </span>
  );
};

export default AwaitingBadge;
