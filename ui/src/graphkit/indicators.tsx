/**
 * GraphKit — indicators. The unified visual language for real captured
 * state (guide §4.4). LAW: every indicator renders REAL values from the
 * records it is handed — progress shown only when the record carries one,
 * activity lit only by real events, honest absence rendered as honest
 * absence. No fabricated percentages, no fake pulses.
 *
 * Icon names verified against the installed lucide-react (all exported).
 */
import React from "react";
import {
  AlertOctagon,
  Check,
  Clock,
  Loader2,
  Pause,
  ShieldX,
  Unlink,
  Link2,
  X,
} from "lucide-react";
import { MODALITY_COLOR } from "../graphRenderers/defaults";
import type { Modality } from "../graphViewTypes";
import type { GKTaskRef } from "./types";

const C_TEXT = "var(--color-text)";
const C_BODY = "var(--color-text-secondary)";
const C_MUTED = "var(--color-text-muted)";

/** ModalityBadge — the universal modality marker (MODALITY_COLOR dot). */
export const ModalityBadge: React.FC<{
  modality: Modality;
  label?: string;
}> = ({ modality, label }) => (
  <span
    title={`Modality: ${modality}`}
    style={{
      display: "inline-flex",
      alignItems: "center",
      gap: 4,
      fontSize: 10.5,
      color: C_BODY,
      whiteSpace: "nowrap",
    }}
  >
    <span
      style={{
        width: 7,
        height: 7,
        borderRadius: 2,
        background: MODALITY_COLOR[modality] ?? "var(--color-text-muted)",
        display: "inline-block",
      }}
    />
    {label ?? modality}
  </span>
);

/** TaskStateIcon — one glyph per real task status, with the honest ones
 * loud: `interrupted` explicitly says it will not resume on its own. */
export const TaskStateIcon: React.FC<{ status: string; size?: number }> = ({ status, size = 13 }) => {
  const s = status.toLowerCase();
  const wrap = (el: React.ReactNode, color: string, title: string) => (
    <span title={title} style={{ display: "inline-flex", color, flexShrink: 0 }}>
      {el}
    </span>
  );
  if (s === "completed")
    return wrap(<Check size={size} />, "#7bd88f", "Completed");
  if (s === "failed")
    return wrap(<X size={size} />, "#ff8a8a", "Failed");
  if (s === "running")
    return wrap(<Loader2 size={size} className="animate-spin" />, "#4fc3f7", "Running");
  if (s === "queued")
    return wrap(<Clock size={size} />, C_MUTED, "Queued");
  if (s === "paused")
    return wrap(<Pause size={size} />, "#d9b45f", "Paused");
  if (s === "interrupted")
    return wrap(<AlertOctagon size={size} />, "#ff9d5f", "Interrupted — the backend restarted or crashed before finishing; it will not complete on its own");
  return wrap(<Clock size={size} />, C_MUTED, status);
};

/** StepPips — one pip per step from the task record's own step statuses.
 * Progression at a glance; NO percentage is invented (the record's own
 * progress field, when present, may be shown beside the pips). */
export const StepPips: React.FC<{ task: GKTaskRef }> = ({ task }) => {
  if (task.stepStatuses.length === 0) return null;
  const color = (st: string) => {
    const s = st.toLowerCase();
    if (s === "completed") return "#7bd88f";
    if (s === "running") return "#4fc3f7";
    if (s === "failed") return "#ff8a8a";
    return "var(--color-border)";
  };
  return (
    <span
      title={task.stepStatuses
        .map((s, i) => `step ${i + 1}: ${s}`)
        .join("\n")}
      style={{ display: "inline-flex", gap: 2, alignItems: "center" }}
    >
      {task.stepStatuses.map((s, i) => (
        <span
          key={i}
          style={{
            width: 6,
            height: 6,
            borderRadius: 1,
            background: color(s),
            display: "inline-block",
          }}
        />
      ))}
      {typeof task.progress === "number" && (
        <span style={{ fontSize: 10, color: C_MUTED, marginLeft: 3 }}>
          {Math.round(task.progress * 100)}%
        </span>
      )}
    </span>
  );
};

/** GateState — real decision-review outcomes. */
export const GateState: React.FC<{ decision: string; size?: number }> = ({ decision, size = 13 }) => {
  const d = decision.toLowerCase();
  if (d === "proceed")
    return (
      <span title="Gate: Proceed" style={{ display: "inline-flex", color: "#7bd88f" }}>
        <Check size={size} />
      </span>
    );
  if (d === "decline")
    return (
      <span title="Gate: Decline" style={{ display: "inline-flex", color: "#ff8a8a" }}>
        <ShieldX size={size} />
      </span>
    );
  return (
    <span
      title="Gate: Review pending — needs a human decision"
      style={{ display: "inline-flex", color: "#d9b45f" }}
    >
      <Clock size={size} />
    </span>
  );
};

/** PeerState — the REAL trust posture, honestly labeled. Connected is NOT
 * trusted: the pairing gate is a separate, operator-controlled fact. */
export const PeerState: React.FC<{
  peerCount: number;
  pairedCount: number;
}> = ({ peerCount, pairedCount }) => {
  if (peerCount === 0 && pairedCount === 0)
    return (
      <span title="P2P: no peers connected" style={{ display: "inline-flex", color: C_MUTED }}>
        <Unlink size={12} />
      </span>
    );
  if (pairedCount === 0)
    return (
      <span
        title={`${peerCount} peer${peerCount === 1 ? "" : "s"} connected — connected is NOT trusted (unpaired)`}
        style={{ display: "inline-flex", color: "#d9b45f" }}
      >
        <Unlink size={12} />
        <span style={{ fontSize: 10, marginLeft: 3 }}>{peerCount} unpaired</span>
      </span>
    );
  return (
    <span
      title={`${pairedCount} paired peer${pairedCount === 1 ? "" : "s"}, ${peerCount} connected`}
      style={{ display: "inline-flex", color: "#7bd88f" }}
    >
      <Link2 size={12} />
      <span style={{ fontSize: 10, marginLeft: 3 }}>{pairedCount} paired</span>
    </span>
  );
};

/** EdgeRealism legend — the one honesty legend every canvas reuses:
 * solid real / dashed derived / dotted inferred. */
export const EdgeRealismLegend: React.FC = () => (
  <span style={{ display: "inline-flex", gap: 10, alignItems: "center", fontSize: 10, color: C_MUTED }}>
    <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
      <svg width="18" height="4"><line x1="0" y1="2" x2="18" y2="2" stroke={C_TEXT} strokeWidth="2" /></svg>
      real
    </span>
    <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
      <svg width="18" height="4"><line x1="0" y1="2" x2="18" y2="2" stroke={C_TEXT} strokeWidth="2" strokeDasharray="4 3" /></svg>
      derived
    </span>
    <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
      <svg width="18" height="4"><line x1="0" y1="2" x2="18" y2="2" stroke={C_TEXT} strokeWidth="2" strokeDasharray="1 3" /></svg>
      inferred
    </span>
  </span>
);

/** ActivityPulse — a real-recency dot: lit only while `activeUntil` is in
 * the future (callers set it from REAL events with a bounded window). */
export const ActivityPulse: React.FC<{ active: boolean; label: string }> = ({ active, label }) => (
  <span
    title={label}
    style={{
      width: 6,
      height: 6,
      borderRadius: "50%",
      background: active ? "#4fc3f7" : "var(--color-border)",
      display: "inline-block",
      animation: active ? "pulse 1s ease-in-out infinite" : undefined,
    }}
  />
);
