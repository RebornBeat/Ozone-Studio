/**
 * F7 — Diff/version history viewer.
 *
 * Real sources, confirmed against source + the live host (2026-09-27), and
 * what each one can and cannot show:
 *
 * 1. Container-level rollback records — `ZSEIQuery::GetVersionHistory{container_id}`
 *    (src/types/zsei.rs:212, IMPLEMENTED in src/zsei/query.rs:188, fed by the
 *    integrity monitor's pre-write snapshots, src/integrity/mod.rs — the same
 *    T-I4 mechanism B18 wired into ZSEI::query's Update/Delete path). Returns
 *    `VersionRecord{version:u64, timestamp:u64, content_hash:Blake3Hash,
 *    change_type:Create|Update|..., rollback_available:bool}`
 *    (src/types/container.rs:402). Verified LIVE against 5 real containers
 *    (workspace/project/jurisdiction roots): every one returns an empty list
 *    today — the mechanism is real and reachable, nothing has been recorded
 *    into it yet for these containers. IMPORTANT LIMITATION, confirmed from
 *    the struct itself: a `VersionRecord` carries only a content HASH, never
 *    the historical content — there is no real data path to build an actual
 *    text diff between two versions from this source, so this view does not
 *    attempt one; it renders the honest record list only.
 *
 * 2. Text-graph node version notes — `TextGraphNode.version: u32` +
 *    `version_notes: Vec<VersionNote>` (assets/pipelines/modalities/text/main.rs:663,
 *    710-711 and others), each note real (`version`, `note`, `step_index?`,
 *    `timestamp`, `change_type`). Reachable today only indirectly: fetch the
 *    project's real text ModalityGraph(s) via `loadGraphData` (B0/B1) and find
 *    the node whose `materialized_path` matches the selected file's path.
 *    Also carries no full historical CONTENT snapshot — `note` is a short
 *    human-readable description of the change, not a diff-able body — so this
 *    section shows the real note list, not a diff either.
 *
 * NOT shown here: task-step version notes (`StepVersionNote`,
 * src/task/mod.rs:634, real and already returned in full by `/task/get`'s
 * `steps[].version_notes`) — that data is task-scoped, not file/container-
 * scoped, and belongs to the separate H3 fork ("Version notes display — real
 * per-step version history"), not this file-selection-driven view.
 */
import React, { useEffect, useMemo, useState } from "react";
import { zseiQuery } from "../../ozoneClient";
import { loadGraphData } from "../../graphViewData";
import type { GraphViewStatus } from "../../graphViewTypes";
import { useSelectedFile } from "../../fileSelection";

// ── Container-level records (source 1) ──────────────────────────────────

interface VersionRecord {
  version: number;
  timestamp: number;
  content_hash: string;
  change_type: string;
  rollback_available: boolean;
}

type ContainerHistoryState =
  | { kind: "loading" }
  | { kind: "error"; message: string }
  | { kind: "ready"; records: VersionRecord[] };

function useContainerVersionHistory(containerId: number | null): ContainerHistoryState {
  const [state, setState] = useState<ContainerHistoryState>({ kind: "loading" });
  useEffect(() => {
    if (containerId === null) return;
    let cancelled = false;
    setState({ kind: "loading" });
    zseiQuery<Record<string, unknown>>({ GetVersionHistory: { container_id: containerId } })
      .then((result) => {
        if (cancelled) return;
        const list = (result as { VersionHistory?: unknown })?.VersionHistory;
        if (!Array.isArray(list)) {
          setState({ kind: "error", message: "Unexpected response shape for GetVersionHistory." });
          return;
        }
        setState({ kind: "ready", records: list as VersionRecord[] });
      })
      .catch((err) => {
        if (!cancelled) setState({ kind: "error", message: err instanceof Error ? err.message : String(err) });
      });
    return () => {
      cancelled = true;
    };
  }, [containerId]);
  return state;
}

// ── Text-graph node notes (source 2) ────────────────────────────────────

interface TextVersionNote {
  version: number;
  note: string;
  step_index?: number | null;
  timestamp: string;
  change_type: string;
}

function findTextNodeNotes(
  status: GraphViewStatus,
  filePath: string | undefined,
): { found: boolean; nodeVersion?: number; notes: TextVersionNote[] } {
  if (!filePath || status.kind !== "ready") return { found: false, notes: [] };
  for (const node of status.data.nodes) {
    if (node.modality !== "text") continue;
    const raw = node.raw as Record<string, unknown>;
    const mp = raw.materialized_path;
    if (typeof mp === "string" && mp === filePath) {
      const notes = Array.isArray(raw.version_notes) ? (raw.version_notes as TextVersionNote[]) : [];
      const nodeVersion = typeof raw.version === "number" ? raw.version : undefined;
      return { found: true, nodeVersion, notes };
    }
  }
  return { found: false, notes: [] };
}

// ── Presentation ──────────────────────────────────────────────────────────

const C_TEXT = "#dfe7f2";
const C_BODY = "#c7d0dc";
const C_MUTED = "#8b98ab";
const C_BORDER = "#1e2836";

function formatTs(ts: number | string): string {
  const n = typeof ts === "string" ? Number(ts) || Date.parse(ts) : ts;
  if (!Number.isFinite(n)) return String(ts);
  const ms = n > 1e12 ? n : n * 1000; // tolerate seconds vs ms
  const d = new Date(ms);
  return Number.isNaN(d.getTime()) ? String(ts) : d.toLocaleString();
}

const SectionTitle: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <div style={{ fontSize: 11, fontWeight: 700, letterSpacing: 0.4, textTransform: "uppercase", color: C_MUTED, margin: "14px 0 6px" }}>
    {children}
  </div>
);

const Empty: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <div style={{ fontSize: 12, color: C_MUTED, fontStyle: "italic", lineHeight: 1.6 }}>{children}</div>
);

export const VersionHistory: React.FC<{ projectId: number | null }> = ({ projectId }) => {
  const [selectedFile] = useSelectedFile();
  const containerState = useContainerVersionHistory(selectedFile?.containerId ?? null);

  const [graphStatus, setGraphStatus] = useState<GraphViewStatus>({ kind: "loading" });
  useEffect(() => {
    if (projectId === null || selectedFile?.modality !== "text") {
      setGraphStatus({ kind: "empty" });
      return;
    }
    return loadGraphData(projectId, setGraphStatus, { modality: "text" });
  }, [projectId, selectedFile?.modality]);

  const textNotes = useMemo(() => findTextNodeNotes(graphStatus, selectedFile?.path), [graphStatus, selectedFile?.path]);

  if (!selectedFile) {
    return (
      <div style={{ padding: 8 }}>
        <Empty>
          Select a file in the Browser sub-tab to see its version history. This system records two real kinds
          today: container-level rollback records (any container) and text-graph node version notes (text
          files only) — task-step version notes live in the Task Viewer instead.
        </Empty>
      </div>
    );
  }

  return (
    <div style={{ padding: 8, fontSize: 12, color: C_BODY }}>
      <div style={{ marginBottom: 4 }}>
        <span style={{ color: C_MUTED }}>File: </span>
        <span style={{ color: C_TEXT }}>{selectedFile.name}</span>
        {selectedFile.path && <span style={{ color: C_MUTED }}> ({selectedFile.path})</span>}
      </div>

      <SectionTitle>Container rollback records</SectionTitle>
      {containerState.kind === "loading" && <Empty>Loading…</Empty>}
      {containerState.kind === "error" && <Empty>Error: {containerState.message}</Empty>}
      {containerState.kind === "ready" && containerState.records.length === 0 && (
        <Empty>
          No rollback records exist yet for this container. This is a real, reachable mechanism
          (GetVersionHistory, backed by the integrity monitor's pre-write snapshots) — it is genuinely empty
          because no tracked update has run against this container yet, not a missing feature.
        </Empty>
      )}
      {containerState.kind === "ready" && containerState.records.length > 0 && (
        <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
          {[...containerState.records]
            .sort((a, b) => b.version - a.version)
            .map((r) => (
              <div key={r.version} style={{ border: `1px solid ${C_BORDER}`, borderRadius: 6, padding: "6px 8px" }}>
                <div style={{ display: "flex", justifyContent: "space-between", color: C_TEXT }}>
                  <span>v{r.version} · {r.change_type}</span>
                  <span style={{ color: C_MUTED }}>{formatTs(r.timestamp)}</span>
                </div>
                <div style={{ color: C_MUTED, fontSize: 11, marginTop: 2 }}>
                  hash {r.content_hash.slice(0, 16)}… · rollback {r.rollback_available ? "available" : "not available"}
                </div>
              </div>
            ))}
          <Empty>
            No diff is shown between versions: a rollback record stores only a content hash for integrity
            verification, never the historical content itself, so no real byte/line diff can be computed from
            this data.
          </Empty>
        </div>
      )}

      <SectionTitle>Text graph version notes</SectionTitle>
      {selectedFile.modality !== "text" ? (
        <Empty>This file isn't a text-modality file — this system only records per-node version notes for text graphs.</Empty>
      ) : graphStatus.kind === "loading" ? (
        <Empty>Loading the project's text graph…</Empty>
      ) : graphStatus.kind === "error" ? (
        <Empty>Error loading the text graph: {graphStatus.message}</Empty>
      ) : !textNotes.found ? (
        <Empty>
          No text-graph node with a materialized_path matching this file was found in the project's text
          graph(s) — either this file hasn't been analysed into the graph yet, or its path doesn't match the
          node's recorded materialized_path.
        </Empty>
      ) : textNotes.notes.length === 0 ? (
        <Empty>The linked graph node exists (version {textNotes.nodeVersion ?? "?"}) but has no recorded version notes yet.</Empty>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
          {[...textNotes.notes]
            .sort((a, b) => b.version - a.version)
            .map((n, i) => (
              <div key={`${n.version}-${i}`} style={{ border: `1px solid ${C_BORDER}`, borderRadius: 6, padding: "6px 8px" }}>
                <div style={{ display: "flex", justifyContent: "space-between", color: C_TEXT }}>
                  <span>v{n.version} · {n.change_type}</span>
                  <span style={{ color: C_MUTED }}>{formatTs(n.timestamp)}</span>
                </div>
                <div style={{ marginTop: 4, color: C_BODY, whiteSpace: "pre-wrap" }}>{n.note}</div>
                {n.step_index !== null && n.step_index !== undefined && (
                  <div style={{ color: C_MUTED, fontSize: 11, marginTop: 2 }}>from step {n.step_index}</div>
                )}
              </div>
            ))}
        </div>
      )}
    </div>
  );
};

export default VersionHistory;
