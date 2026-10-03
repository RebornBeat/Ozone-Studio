/**
 * A1 — Live orchestration status panel.
 *
 * Real signal only, over the already-extended `graphEventClient.ts`:
 *  - `orchestration_stage` frames (src/orchestration_events.rs, emitted from
 *    the real record_stage/record_stage_timed choke point every one of the
 *    real 15 `/orchestrate` stages passes through) — real stage number,
 *    name, success flag, and dynamic `summary` text already produced by
 *    that stage. This is the only live signal for stages with no
 *    subprocess pipeline (Build AMT, Blueprint Assignment, Zero-Shot
 *    Simulation).
 *  - `pipeline_progress` frames (src/pipeline/executor.rs) — every real
 *    subprocess pipeline invocation (TextAnalysisPipeline, WorkspaceTab,
 *    the prompt pipeline, ...). `progress_percent` is honestly only ever 0
 *    (Running) or 100 (terminal) — never rendered as a smoothly-filling
 *    bar, since that would imply granularity that doesn't exist.
 *
 * The operator has explicitly kept the full 15-stage depth as-is (deliberate
 * thorough zero-shot knowledge construction, not something to shortcut) —
 * this panel only makes that real process observable, never implies it's
 * faster or different than it actually is. A "skipped" stage is inferred
 * honestly from a real gap in the sequence (e.g. stage 4 arrives with no
 * stage 3 having fired), not hardcoded from which stages are "usually" off.
 *
 * Known simplification, not a correctness guarantee: `/orchestrate` is
 * currently always called with hardcoded `user_id:1, device_id:1`
 * (components/MetaPortion.tsx) — frames are filtered to those ids. A real
 * per-request id would be needed for genuine multi-user/multi-tab isolation.
 */
import React, { useEffect, useRef, useState } from "react";
import {
  getGraphEventClient,
  OrchestrationStageFrame,
  PipelineProgressFrame,
} from "../graphEventClient";

const FILTER_USER_ID = 1;
const FILTER_DEVICE_ID = 1;

// Real stage numbers/names, per the actual record_stage/record_stage_timed
// call sites in src/orchestrator/{stages,mod,jurisdiction,amt,response}.rs
// — not guessed, not renumbered on this side. Found live 2026-09-29: the
// backend previously REUSED numbers for different logical stages (3 meant
// "Gather Methodologies" AND "Blueprint Assignment" depending on code path;
// 4 meant "Initial Graph Creation" AND "Zero-Shot Simulation"; 5 meant
// "Build AMT" AND "Consciousness Gate"; "Post-execution" was 10 on the
// consciousness-enabled path but 13 when disabled) — since this panel keys
// rows by stage number, a later stage's frame silently overwrote an
// earlier, unrelated stage's row instead of appearing as its own row. Fixed
// on the backend (each logical stage now has one unique number, 0-13); this
// list must stay in lockstep with that renumbering, not the old one.
const STAGE_ORDER: { stage: number; name: string }[] = [
  { stage: 0, name: "Jurisdiction Gate" },
  { stage: 1, name: "Input Capture" },
  { stage: 2, name: "Text Normalization" },
  { stage: 3, name: "Gather Methodologies" },
  { stage: 4, name: "Initial Graph Creation" },
  { stage: 5, name: "Build AMT" },
  { stage: 6, name: "Blueprint Assignment" },
  { stage: 7, name: "Zero-Shot Simulation" },
  { stage: 8, name: "Consciousness Gate" },
  { stage: 9, name: "Task Creation" },
  { stage: 10, name: "Step Execution" },
  { stage: 11, name: "Result Collection" },
  { stage: 12, name: "Post-execution Consciousness" },
  { stage: 13, name: "Response Delivery" },
];

type StageStatus = "pending" | "current" | "completed" | "failed" | "skipped";

interface StageRow {
  stage: number;
  name: string;
  status: StageStatus;
  summary?: string;
  durationMs?: number;
}

function freshRows(): StageRow[] {
  return STAGE_ORDER.map((s) => ({ stage: s.stage, name: s.name, status: "pending" }));
}

function formatElapsed(ms: number): string {
  const totalSec = Math.floor(ms / 1000);
  const m = Math.floor(totalSec / 60);
  const s = totalSec % 60;
  return `${m}:${String(s).padStart(2, "0")}`;
}

function truncate(s: string, max: number): string {
  return s.length > max ? `${s.slice(0, max - 1)}…` : s;
}

export const OrchestrationStatusPanel: React.FC<{ isRunning: boolean }> = ({ isRunning }) => {
  const [rows, setRows] = useState<StageRow[]>(freshRows());
  const [recentPipelines, setRecentPipelines] = useState<PipelineProgressFrame[]>([]);
  const [expanded, setExpanded] = useState(false);
  const [elapsedMs, setElapsedMs] = useState(0);
  const startRef = useRef<number>(0);

  useEffect(() => {
    if (!isRunning) return;
    // Fresh run: reset everything so a prior run's state never bleeds in.
    setRows(freshRows());
    setRecentPipelines([]);
    startRef.current = Date.now();
    setElapsedMs(0);

    const client = getGraphEventClient();
    client.connect();

    const tick = window.setInterval(() => setElapsedMs(Date.now() - startRef.current), 1000);

    const offStage = client.onOrchestrationStage((frame: OrchestrationStageFrame) => {
      if (frame.user_id !== FILTER_USER_ID || frame.device_id !== FILTER_DEVICE_ID) return;
      setRows((prev) => {
        const idx = prev.findIndex((r) => r.stage === frame.stage);
        if (idx === -1) return prev; // a real stage number outside our known list — ignore, don't fabricate a row
        const next = prev.map((r) => ({ ...r }));
        next[idx].status = frame.success ? "completed" : "failed";
        next[idx].summary = frame.summary;
        next[idx].durationMs = frame.duration_ms;
        // Any stage strictly between the previously-latest completed one and
        // this one that never got its own frame is honestly "skipped" —
        // inferred from the real gap, not a hardcoded skip list.
        for (let i = 0; i < idx; i++) {
          if (next[i].status === "pending") next[i].status = "skipped";
        }
        // The next pending stage (if any) becomes "current" — the real
        // process is now working on it, even though we won't hear from it
        // until it completes.
        for (let i = idx + 1; i < next.length; i++) {
          if (next[i].status === "pending") {
            next[i].status = "current";
            break;
          }
        }
        return next;
      });
    });

    const offPipeline = client.onPipelineProgress((frame: PipelineProgressFrame) => {
      setRecentPipelines((prev) => [frame, ...prev].slice(0, 3));
    });

    return () => {
      window.clearInterval(tick);
      offStage();
      offPipeline();
    };
  }, [isRunning]);

  if (!isRunning) return null;

  const completedCount = rows.filter((r) => r.status === "completed" || r.status === "skipped").length;
  const currentRow = rows.find((r) => r.status === "current");

  return (
    <div
      role="status"
      aria-label="Orchestration progress"
      style={{
        display: "flex",
        flexDirection: "column",
        gap: 4,
        fontSize: 11.5,
        color: "var(--color-text-muted)",
        background: "#101724",
        border: "1px solid var(--color-border-faint)",
        borderRadius: 8,
        padding: "6px 8px",
        marginBottom: 4,
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <span
          aria-hidden="true"
          style={{
            width: 6,
            height: 6,
            borderRadius: "50%",
            background: "#6ec3ff",
            animation: "pulse 1.2s ease-in-out infinite",
            flex: "none",
          }}
        />
        {/* aria-live scoped to just this text (not the whole role="status"
            container above) so a screen reader announces stage/count
            changes without re-reading the expand button on every tick. */}
        <span aria-live="polite" style={{ color: "var(--color-text)", fontWeight: 600 }}>
          {currentRow ? currentRow.name : "Working…"}
        </span>
        <span style={{ color: "var(--color-text-muted)" }}>
          {completedCount}/{rows.length} stages · {formatElapsed(elapsedMs)}
        </span>
        <button
          onClick={() => setExpanded((e) => !e)}
          aria-expanded={expanded}
          aria-controls="orchestration-stage-list"
          style={{
            marginLeft: "auto",
            background: "none",
            border: "none",
            color: "#6ec3ff",
            cursor: "pointer",
            fontSize: 11,
            padding: "2px 4px",
          }}
        >
          {expanded ? "▾ hide" : "▸ details"}
        </button>
      </div>

      {recentPipelines.length > 0 && (
        <div style={{ display: "flex", gap: 10, flexWrap: "wrap", paddingLeft: 14 }}>
          {recentPipelines.map((p, i) => (
            <span
              key={`${p.execution_id}-${i}`}
              style={{
                opacity: 1 - i * 0.3,
                color: p.status === "Running" ? "#6ec3ff" : p.status === "Failed" ? "#ff8a8a" : "#8fe38f",
                fontSize: 11,
                transition: "opacity 0.4s ease",
              }}
            >
              {p.status === "Running" ? "→" : p.status === "Failed" ? "✗" : "✓"} {p.pipeline_name}
            </span>
          ))}
        </div>
      )}

      {expanded && (
        <div
          id="orchestration-stage-list"
          role="list"
          aria-label="Orchestration stages"
          style={{ display: "flex", flexDirection: "column", gap: 2, marginTop: 4, paddingLeft: 14 }}
        >
          {rows.map((r) => (
            <div
              key={r.stage}
              role="listitem"
              aria-label={`Stage ${r.stage}, ${r.name}: ${r.status}${r.summary ? `. ${r.summary}` : ""}`}
              style={{
                display: "flex",
                alignItems: "baseline",
                gap: 6,
                opacity: r.status === "pending" ? 0.4 : 1,
                transition: "opacity 0.4s ease, color 0.4s ease",
              }}
            >
              <span
                style={{
                  width: 12,
                  flex: "none",
                  color:
                    r.status === "completed"
                      ? "#8fe38f"
                      : r.status === "failed"
                        ? "#ff8a8a"
                        : r.status === "current"
                          ? "#6ec3ff"
                          : "var(--color-text-muted)",
                }}
              >
                {r.status === "completed" ? "✓" : r.status === "failed" ? "✗" : r.status === "skipped" ? "—" : r.status === "current" ? "●" : "○"}
              </span>
              <span style={{ color: r.status === "current" ? "var(--color-text)" : "var(--color-text-secondary)", minWidth: 90 }}>
                {r.name}
              </span>
              {r.summary && (
                <span style={{ color: "var(--color-text-muted)", fontSize: 10.5 }} title={r.summary}>
                  {truncate(r.summary, 70)}
                  {typeof r.durationMs === "number" && r.durationMs > 0 ? ` (${(r.durationMs / 1000).toFixed(1)}s)` : ""}
                </span>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

export default OrchestrationStatusPanel;
