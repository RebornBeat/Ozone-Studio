/**
 * CreationActivity — the live "fun to know" layer (operator: when
 * methodologies / tools / MCPs / pipelines / graphs get created or
 * aggregated, the user should SEE it happen with a nice animation).
 *
 * Subscribes to the real graph-event WebSocket (graphEventClient.ts — the
 * same /ws push the host broadcasts at the universal choke point) and
 * surfaces animated chips for creation events on the surfaces the user
 * cares about: Methodology, ModalityGraph, Task, CoordinationEvent. Each
 * chip: type icon + scope keywords, fade-slide entry with a glow pulse,
 * auto-expiry. Events are real (the host's own choke-point broadcast) —
 * nothing here is simulated.
 */
import React, { useEffect, useRef, useState } from "react";
import { getGraphEventClient, GraphEventFrame } from "../graphEventClient";

interface Activity {
  key: number;
  icon: string;
  label: string;
  detail: string;
}

const TYPE_STYLES: Record<string, { icon: string; label: string }> = {
  Methodology: { icon: "🧭", label: "Methodology created" },
  ModalityGraph: { icon: "🕸️", label: "Modality graph created" },
  Task: { icon: "⚙️", label: "Task created" },
  CoordinationEvent: { icon: "🤝", label: "Coordination finding" },
};

const ACTIVITY_TYPES = new Set(Object.keys(TYPE_STYLES));
const EXPIRE_MS = 9000;
const MAX_VISIBLE = 4;

export default function CreationActivity() {
  const [items, setItems] = useState<Activity[]>([]);
  const nextKey = useRef(1);

  useEffect(() => {
    const off = getGraphEventClient().onEvent((frame: GraphEventFrame) => {
      if (frame.event !== "created" || !ACTIVITY_TYPES.has(frame.container_type)) return;
      const style = TYPE_STYLES[frame.container_type] ?? { icon: "✦", label: frame.container_type };
      const detail =
        (frame.scope_keywords ?? [])
          .filter((k) => !k.startsWith("ws:") && !k.startsWith("proj:"))
          .slice(0, 3)
          .join(" · ") || `#${frame.container_id}`;
      const activity: Activity = {
        key: nextKey.current++,
        icon: style.icon,
        label: style.label,
        detail,
      };
      setItems((prev) => [activity, ...prev].slice(0, MAX_VISIBLE));
      window.setTimeout(() => {
        setItems((prev) => prev.filter((p) => p.key !== activity.key));
      }, EXPIRE_MS);
    });
    return off;
  }, []);

  if (items.length === 0) return null;

  return (
    <React.Fragment>
      <style>{`
        @keyframes oz-activity-in {
          0%   { opacity: 0; transform: translateY(-6px) scale(0.97); }
          60%  { opacity: 1; transform: translateY(0) scale(1.02); }
          100% { opacity: 1; transform: translateY(0) scale(1); }
        }
        @keyframes oz-activity-glow {
          0%, 100% { box-shadow: 0 0 0 rgba(122, 162, 247, 0); }
          40%      { box-shadow: 0 0 12px rgba(122, 162, 247, 0.45); }
        }
      `}</style>
      <div style={{ display: "flex", flexDirection: "column", gap: 6, margin: "8px 0 12px" }}>
        {items.map((a) => (
          <div
            key={a.key}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 8,
              background: "var(--color-surface-2, #1c2430)",
              border: "1px solid #2a3444",
              borderLeft: "3px solid #7aa2f7",
              borderRadius: 6,
              padding: "6px 10px",
              color: "#e8eef6",
              fontSize: 12,
              animation: "oz-activity-in 260ms ease-out, oz-activity-glow 1.4s ease-out",
            }}
          >
            <span style={{ fontSize: 14 }}>{a.icon}</span>
            <span style={{ fontWeight: 600 }}>{a.label}</span>
            <span style={{ color: "#8a99ab", marginLeft: "auto", fontSize: 11, textAlign: "right" }}>
              {a.detail}
            </span>
          </div>
        ))}
      </div>
    </React.Fragment>
  );
}
