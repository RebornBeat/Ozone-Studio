/**
 * GraphKit — primitives. The shared node/inspector/link components every
 * lens, engine, and file view renders through (guide §4.2). The silo-killer
 * is CrossLinkStrip: for ANY entity it lists the entity's real cross-links
 * as chips that navigate through the spine — the same strip in the engines,
 * the graph inspector, and the file viewer means no view is an island.
 */
import React from "react";
import { MODALITY_COLOR } from "../graphRenderers/defaults";
import type { Modality } from "../graphViewTypes";
import { useGraphKit } from "./graphProvider";
import type { GKStatus } from "./types";
import { ModalityBadge } from "./indicators";

const C_TEXT = "var(--color-text)";
const C_BODY = "var(--color-text-secondary)";
const C_MUTED = "var(--color-text-muted)";
const C_BORDER = "var(--color-border-faint)";
const C_ERROR = "#ff8a8a";

/** HonestState — the codified loading/error/empty contract. `empty` carries
 * a real WHY; error carries the real message; loading is a quiet spine. */
export const HonestState: React.FC<{ status: GKStatus }> = ({ status }) => {
  if (status.kind === "loading")
    return <div style={{ color: C_MUTED, fontSize: 12.5 }}>Loading…</div>;
  if (status.kind === "error")
    return <div style={{ color: C_ERROR, fontSize: 12.5 }}>Error: {status.message}</div>;
  if (status.kind === "empty")
    return <div style={{ color: C_MUTED, fontSize: 12.5, lineHeight: 1.5 }}>{status.why}</div>;
  return null;
};

/** NodeCard — the ONE entity card contract: modality badge, title, isReal
 * honesty, confidence when real. Extra rows come from the lens, not here. */
export const NodeCard: React.FC<{
  label: string;
  nodeType?: string;
  modality?: Modality | null;
  isReal?: boolean;
  confidence?: number | null;
  onOpen?: () => void;
}> = ({ label, nodeType, modality, isReal = true, confidence, onOpen }) => (
  <div
    onClick={onOpen}
    style={{
      border: `1px solid ${C_BORDER}`,
      borderRadius: 8,
      padding: "8px 10px",
      cursor: onOpen ? "pointer" : "default",
      display: "flex",
      flexDirection: "column",
      gap: 4,
    }}
  >
    <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
      {modality && <ModalityBadge modality={modality} />}
      <span style={{ color: C_TEXT, fontSize: 12.5, fontWeight: 600, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis" }}>
        {label}
      </span>
      {!isReal && (
        <span title="Derived — not directly stored" style={{ fontSize: 9.5, color: C_MUTED, border: `1px dashed ${C_BORDER}`, borderRadius: 3, padding: "0 4px" }}>
          derived
        </span>
      )}
    </div>
    <div style={{ display: "flex", gap: 8, fontSize: 11, color: C_MUTED }}>
      {nodeType && <span>{nodeType}</span>}
      {typeof confidence === "number" && <span>confidence {confidence.toFixed(2)}</span>}
    </div>
  </div>
);

/** CrossLinkStrip — THE silo-killer. For a container id, lists the real
 * cross-links (both directions, from the substrate) as chips: target label,
 * relation type, modality badge — each chip navigates via `onNavigate`.
 * `relationFilter` lets a lens scope to the relation kinds it means
 * (default: all — the graph-first default). */
export const CrossLinkStrip: React.FC<{
  containerId: number;
  onNavigate: (targetContainer: number) => void;
  max?: number;
}> = ({ containerId, onNavigate, max = 8 }) => {
  const links = useGraphKit((s) => s.crossLinksByContainer[containerId]);
  if (!links || links.length === 0) return null;
  const shown = links.slice(0, max);
  return (
    <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginTop: 4 }}>
      {shown.map((link, i) => {
        const isCrossModal =
          link.targetModality !== null && link.relationType === "SimilarTo";
        return (
          <button
            key={`${link.targetContainer}:${link.relationType}:${i}`}
            onClick={() => onNavigate(link.targetContainer)}
            title={`${link.relationType} → ${link.targetLabel ?? `container ${link.targetContainer}`} (via ${link.discovered_via}, confidence ${link.confidence.toFixed(2)})`}
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: 4,
              fontSize: 10.5,
              color: isCrossModal ? "#e879f9" : C_BODY,
              background: "transparent",
              border: `1px solid ${isCrossModal ? "rgba(232,121,249,0.4)" : C_BORDER}`,
              borderRadius: 10,
              padding: "1px 7px",
              cursor: "pointer",
            }}
          >
            {link.targetModality && (
              <span
                style={{
                  width: 6,
                  height: 6,
                  borderRadius: 2,
                  background: MODALITY_COLOR[link.targetModality],
                  display: "inline-block",
                }}
              />
            )}
            {link.targetLabel ?? `#${link.targetContainer}`}
            <span style={{ color: C_MUTED }}>{link.relationType}</span>
          </button>
        );
      })}
      {links.length > shown.length && (
        <span style={{ fontSize: 10, color: C_MUTED, alignSelf: "center" }}>
          +{links.length - shown.length} more links
        </span>
      )}
    </div>
  );
};

/** ModalityWheel — the 27-modality switcher, one component for every
 * surface (workspace header, graph filter, fabric). Reads its palette from
 * MODALITY_COLOR: single source, auto-extending. */
export const ModalityWheel: React.FC<{
  value: Modality | "all";
  onChange: (m: Modality | "all") => void;
  available?: Modality[]; // modalities actually present in the substrate
}> = ({ value, onChange, available }) => {
  const all = Object.keys(MODALITY_COLOR) as Modality[];
  const present = new Set(available ?? []);
  return (
    <div style={{ display: "flex", flexWrap: "wrap", gap: 3 }}>
      <button
        onClick={() => onChange("all")}
        title="All modalities"
        style={wheelBtn(value === "all", false)}
      >
        all
      </button>
      {all.map((m) => (
        <button
          key={m}
          onClick={() => onChange(m)}
          title={present.size > 0 && !present.has(m) ? `${m} — no graphs in this project yet` : m}
          style={wheelBtn(value === m, present.size > 0 && !present.has(m))}
        >
          <span
            style={{
              width: 6,
              height: 6,
              borderRadius: 2,
              background: MODALITY_COLOR[m],
              display: "inline-block",
            }}
          />
          {m}
        </button>
      ))}
    </div>
  );
};

function wheelBtn(active: boolean, dimmed: boolean): React.CSSProperties {
  return {
    display: "inline-flex",
    alignItems: "center",
    gap: 3,
    fontSize: 10,
    color: active ? "var(--color-text)" : C_MUTED,
    background: active ? "rgba(79,195,247,0.12)" : "transparent",
    border: `1px solid ${active ? "rgba(79,195,247,0.4)" : "transparent"}`,
    borderRadius: 8,
    padding: "1px 6px",
    cursor: "pointer",
    opacity: dimmed ? 0.45 : 1,
  };
}
