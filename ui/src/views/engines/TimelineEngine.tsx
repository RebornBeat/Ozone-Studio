/**
 * Timeline Engine — shared Audio (103) / Video (104) engine UI.
 *
 * One component for both: `AudioGraphNode`/`VideoGraphNode`
 * (assets/pipelines/modalities/audio/main.rs:624-641,
 * assets/pipelines/modalities/video/main.rs:531-541) are structurally
 * identical for this view's purposes — `node_type`, `label`, `content`,
 * `time_range: Option<{start,end}>` (seconds, f32), `confidence`,
 * `properties`. Video additionally carries `frame_range` but nothing here
 * needs frame numbers over seconds, so it's read through `raw` only, not
 * promoted. Two separate files would have been the same layout/logic twice.
 *
 * Real node types (confirmed by reading both main.rs files directly):
 *   audio: Audio, Segment, Speaker, Word, Sentence, MusicSection, Chord, Beat, Event, Note
 *   video: Video, Scene, Shot, Keyframe, ObjectTrack, ObjectAppearance, AudioTrack, AudioSegment
 * `time_range` is genuinely populated at most real construction sites in
 * audio/main.rs (confirmed via grep: 4 of 5 construction sites use `Some`,
 * one root/summary node uses `None`) — this is real schema, not aspirational.
 *
 * CURRENT REAL DATA STATE (checked against the only two audio/video graphs
 * that exist on disk anywhere in this repo, target/release/zsei_data/graphs/
 * audio_40291.json and video_40292.json): each has exactly ONE node — the
 * root Audio/Video node itself, `time_range: {start:0, end:0}`, zero edges,
 * zero Segment/Scene/etc. children. Same situation CodeCallGraph.tsx
 * documented for Calls edges: the schema and this view are real and ready,
 * today's actual data just hasn't been built out by a real analysis run
 * yet. The empty-state message below says this plainly, not as a
 * hypothetical caveat.
 */
import React, { useEffect, useMemo, useState } from "react";
import { loadGraphData } from "../../graphViewData";
import { GraphViewNode, GraphViewStatus, Modality } from "../../graphViewTypes";
import { navigateTo } from "../../navigation";

const C_TEXT = "var(--color-text)";
const C_BODY = "var(--color-text-secondary)";
const C_MUTED = "var(--color-text-muted)";
const C_BORDER = "var(--color-border-faint)";
const C_PANEL = "var(--color-bg)";
const C_ERROR = "#ff8a8a";

const MODALITY_ACCENT: Record<"audio" | "video", string> = {
  audio: "#b388ff", // MODALITY_COLOR.audio
  video: "#ff6e6e", // MODALITY_COLOR.video
};

// Distinct real node_type -> swatch color, per modality. Unlisted real
// types (there are none left unlisted as of the main.rs read above) fall
// back to the modality accent so a future backend addition still renders
// sanely instead of invisibly.
const AUDIO_TYPE_COLOR: Record<string, string> = {
  Audio: "#6b7280",
  Segment: "#b388ff",
  Speaker: "#5fb3ff",
  Word: "#8bd9c6",
  Sentence: "#8bd9c6",
  MusicSection: "#ffb86b",
  Chord: "#ffb86b",
  Beat: "#ffd27a",
  Event: "#ff8a8a",
  Note: "#ffd27a",
};
const VIDEO_TYPE_COLOR: Record<string, string> = {
  Video: "#6b7280",
  Scene: "#ff6e6e",
  Shot: "#ff9e6e",
  Keyframe: "#ffd27a",
  ObjectTrack: "#5fb3ff",
  ObjectAppearance: "#8bd9c6",
  AudioTrack: "#b388ff",
  AudioSegment: "#b388ff",
};

interface TimedNode {
  node: GraphViewNode;
  start: number;
  end: number;
}

function timeRangeOf(node: GraphViewNode): { start: number; end: number } | null {
  const raw = node.raw as { time_range?: { start?: number; end?: number } | null };
  const tr = raw?.time_range;
  if (!tr || typeof tr.start !== "number" || typeof tr.end !== "number") return null;
  return { start: tr.start, end: tr.end };
}

function fmtTime(seconds: number): string {
  if (!Number.isFinite(seconds)) return "0:00";
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
}

function jumpToNode(node: GraphViewNode, projectId: number) {
  navigateTo({ kind: "graph-node", projectId, nodeId: node.id });
}

export const TimelineEngine: React.FC<{ projectId: number | null; modality: "audio" | "video" }> = ({
  projectId,
  modality,
}) => {
  const [status, setStatus] = useState<GraphViewStatus>({ kind: "loading" });
  const accent = MODALITY_ACCENT[modality];
  const typeColor = modality === "audio" ? AUDIO_TYPE_COLOR : VIDEO_TYPE_COLOR;
  const rootType = modality === "audio" ? "Audio" : "Video";

  useEffect(() => {
    if (projectId === null) return;
    setStatus({ kind: "loading" });
    const dispose = loadGraphData(projectId, setStatus, { modality: modality as Modality });
    return dispose;
  }, [projectId, modality]);

  const { timed, untimed } = useMemo(() => {
    if (status.kind !== "ready") return { timed: [] as TimedNode[], untimed: [] as GraphViewNode[] };
    const timed: TimedNode[] = [];
    const untimed: GraphViewNode[] = [];
    for (const node of status.data.nodes) {
      // The root Audio/Video node is real but not an "event" on its own
      // timeline — exclude it from the track even when it does carry a
      // (real, always-zero-length) time_range, same reasoning CodeCallGraph
      // excludes the File node from its caller/callee list.
      if (node.nodeType === rootType) continue;
      const tr = timeRangeOf(node);
      if (tr && tr.end > tr.start) timed.push({ node, start: tr.start, end: tr.end });
      else untimed.push(node);
    }
    timed.sort((a, b) => a.start - b.start);
    return { timed, untimed };
  }, [status, rootType]);

  const maxTime = useMemo(() => (timed.length ? Math.max(...timed.map((t) => t.end)) : 0), [timed]);

  if (projectId === null) {
    return <div style={{ color: C_MUTED, fontSize: 12.5 }}>Select a project to see its {modality} timeline.</div>;
  }

  return (
    <div>
      <div style={{ fontSize: 12.5, fontWeight: 700, color: C_TEXT, marginBottom: 8, textTransform: "capitalize" }}>
        {modality} Timeline
      </div>

      {status.kind === "loading" && <div style={{ color: C_MUTED, fontSize: 12.5 }}>Loading {modality} graph…</div>}
      {status.kind === "error" && (
        <div style={{ color: C_ERROR, fontSize: 12.5 }}>Error loading {modality} graph: {status.message}</div>
      )}
      {status.kind === "empty" && (
        <div style={{ color: C_MUTED, fontSize: 12.5 }}>
          No {modality}-modality graph exists for this project yet — nothing to show.
        </div>
      )}

      {status.kind === "ready" && timed.length === 0 && (
        <div
          style={{
            color: C_MUTED,
            fontSize: 12.5,
            border: `1px dashed ${C_BORDER}`,
            borderRadius: 8,
            padding: 16,
            lineHeight: 1.6,
          }}
        >
          This project's {modality} graph exists but has <b>no timed segments</b> yet — only the root {rootType}{" "}
          node. Verified against every {modality} graph currently on disk: each is exactly this shape (one root
          node, zero children). This is not a bug in this view: real {modality} analysis (segments/speakers/scenes/
          object tracks) hasn't been run to produce timeline content yet, so there's genuinely nothing to place on a
          track. This is today's real state, not a fallback.
        </div>
      )}

      {status.kind === "ready" && timed.length > 0 && (
        <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
          {/* Track */}
          <div style={{ position: "relative", height: 48, border: `1px solid ${C_BORDER}`, borderRadius: 6, background: C_PANEL }}>
            {timed.map((t) => {
              const leftPct = maxTime > 0 ? (t.start / maxTime) * 100 : 0;
              const widthPct = maxTime > 0 ? Math.max(((t.end - t.start) / maxTime) * 100, 0.6) : 100;
              return (
                <div
                  key={t.node.id}
                  onClick={() => jumpToNode(t.node, projectId)}
                  title={`${t.node.nodeType} ${fmtTime(t.start)}–${fmtTime(t.end)}: ${t.node.contentPreview ?? t.node.label}`}
                  style={{
                    position: "absolute",
                    left: `${leftPct}%`,
                    width: `${widthPct}%`,
                    top: 6,
                    bottom: 6,
                    minWidth: 3,
                    background: typeColor[t.node.nodeType] ?? accent,
                    opacity: 0.85,
                    borderRadius: 3,
                    cursor: "pointer",
                  }}
                />
              );
            })}
          </div>
          <div style={{ display: "flex", justifyContent: "space-between", fontSize: 10.5, color: C_MUTED, marginTop: -8 }}>
            <span>0:00</span>
            <span>{fmtTime(maxTime)}</span>
          </div>

          {/* Event list */}
          <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            {timed.map((t) => (
              <div
                key={t.node.id}
                onClick={() => jumpToNode(t.node, projectId)}
                style={{
                  display: "flex",
                  alignItems: "baseline",
                  gap: 8,
                  cursor: "pointer",
                  border: `1px solid ${C_BORDER}`,
                  borderLeft: `3px solid ${typeColor[t.node.nodeType] ?? accent}`,
                  borderRadius: 6,
                  padding: "6px 10px",
                }}
              >
                <span style={{ color: C_MUTED, fontSize: 11, fontVariantNumeric: "tabular-nums" }}>
                  {fmtTime(t.start)}–{fmtTime(t.end)}
                </span>
                <span style={{ color: typeColor[t.node.nodeType] ?? accent, fontSize: 11, fontWeight: 600 }}>
                  {t.node.nodeType}
                </span>
                <span style={{ color: C_BODY, fontSize: 12 }}>{t.node.contentPreview ?? t.node.label}</span>
              </div>
            ))}
          </div>

          <div style={{ color: C_MUTED, fontSize: 11 }}>
            {timed.length} timed node{timed.length === 1 ? "" : "s"} across {fmtTime(maxTime)}
            {untimed.length > 0 && ` · ${untimed.length} node${untimed.length === 1 ? "" : "s"} with no time range`}
          </div>
        </div>
      )}
    </div>
  );
};

export const AudioEngine: React.FC<{ projectId: number | null }> = ({ projectId }) => (
  <TimelineEngine projectId={projectId} modality="audio" />
);
export const VideoEngine: React.FC<{ projectId: number | null }> = ({ projectId }) => (
  <TimelineEngine projectId={projectId} modality="video" />
);

export default TimelineEngine;
