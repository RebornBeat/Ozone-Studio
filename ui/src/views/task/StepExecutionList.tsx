/**
 * H2 — Pipeline execution list per step.
 *
 * Real timing/model/token data straight from `/task/get`'s per-step payload
 * (`TaskStepData`, src/task/mod.rs:490-524) — fields `started_at`/
 * `completed_at`/`stages_completed`/`current_stage` are real and already
 * serialized by the host today, but `TaskDetailPanel.tsx`'s own narrower
 * `TaskStep` type doesn't declare them, so its step table never shows real
 * per-step duration. This view reads the same `task.steps` array with a
 * fuller type and adds a real duration column.
 *
 * INVESTIGATED, NOT BUILT — no real step-level call-site correlation exists:
 * `ThinkingEntry` (src/orchestrator/mod.rs:276) — the "thinking cycle" raw
 * LLM calls already shown in TaskDetailPanel — carries NO `step_index` or
 * timestamp, only a free-text `stage` label; it's a flat, unordered-by-step
 * list, so a call cannot be reliably tied back to one step. `eval_tokens_per_sec`
 * /`prompt_eval_tokens_per_sec`/`load_time_ms` are real but BitNet-only (per
 * that struct's own doc comment) — None for API-backed calls, never
 * fabricated here. S11 (`zero_shot_calls.jsonl`, via `fetchZeroShotCalls`)
 * carries `amt_container_id`/`blueprint_id`/`project_id` but NO task id or
 * step index (confirmed against the real 12-field row shape in
 * `../../data/captureData.ts`) — also not joinable to a specific step.
 * Both are shown as separate, honestly-labeled supplementary lists instead
 * of a fabricated join.
 */
import React from "react";

interface RealTaskStep {
  step_index: number;
  action: string;
  pipeline_id: number;
  status: string;
  tokens_used: number;
  model_used?: string | null;
  error?: string | null;
  started_at?: number | null;
  completed_at?: number | null;
  current_stage?: string | null;
  stages_completed?: string[];
}

interface RealThinkingEntry {
  stage: string;
  raw_response: string;
  tokens_used?: number | null;
  model_used?: string | null;
  eval_tokens_per_sec?: number | null;
  prompt_eval_tokens_per_sec?: number | null;
  load_time_ms?: number | null;
}

function normalizeStatus(raw?: string | null): string {
  if (!raw) return "";
  return raw.replace(/^"+|"+$/g, "");
}

// started_at/completed_at are unix SECONDS (verified live: task 10's real
// step returned started_at=completed_at=1789326309, a 10-digit second-scale
// value, not milliseconds) — a fast single-call step often has both fields
// equal, which is real sub-second completion, not a missing/broken value.
function formatDuration(startedAt?: number | null, completedAt?: number | null): string {
  if (!startedAt || !completedAt || completedAt < startedAt) return "—";
  const secs = completedAt - startedAt;
  if (secs === 0) return "<1s";
  if (secs < 60) return `${secs}s`;
  const m = Math.floor(secs / 60);
  const s = secs % 60;
  return `${m}m ${s}s`;
}

export const StepExecutionList: React.FC<{ task: any }> = ({ task }) => {
  const steps: RealTaskStep[] = task?.steps ?? [];
  const thinking: RealThinkingEntry[] = task?.thinking_log ?? [];

  if (steps.length === 0) {
    return <div style={{ fontSize: 12, color: "#8b98ab" }}>No steps recorded yet.</div>;
  }

  return (
    <div style={{ fontSize: 12.5, color: "#c7d0dc" }}>
      <div style={{ fontWeight: 700, color: "#dfe7f2", marginBottom: 6 }}>
        Pipeline execution list ({steps.length} step{steps.length === 1 ? "" : "s"})
      </div>
      <div style={{ overflowX: "auto" }}>
        <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 11.5 }}>
          <thead>
            <tr style={{ color: "#8b98ab", textAlign: "left" }}>
              <th style={{ padding: "3px 8px" }}>#</th>
              <th style={{ padding: "3px 8px" }}>Pipeline</th>
              <th style={{ padding: "3px 8px" }}>Status</th>
              <th style={{ padding: "3px 8px" }}>Model</th>
              <th style={{ padding: "3px 8px" }}>Tokens</th>
              <th style={{ padding: "3px 8px" }}>Duration</th>
              <th style={{ padding: "3px 8px" }}>Stage</th>
            </tr>
          </thead>
          <tbody>
            {steps.map((s) => (
              <tr key={s.step_index} style={{ borderTop: "1px solid #1e2836" }}>
                <td style={{ padding: "3px 8px", fontFamily: "monospace" }}>{s.step_index}</td>
                <td style={{ padding: "3px 8px", fontFamily: "monospace" }}>{s.pipeline_id}</td>
                <td style={{ padding: "3px 8px" }}>{normalizeStatus(s.status) || "—"}</td>
                <td style={{ padding: "3px 8px", fontFamily: "monospace" }}>{s.model_used ?? "—"}</td>
                <td style={{ padding: "3px 8px", fontFamily: "monospace" }}>{s.tokens_used}</td>
                <td style={{ padding: "3px 8px", fontFamily: "monospace" }}>
                  {formatDuration(s.started_at, s.completed_at)}
                </td>
                <td style={{ padding: "3px 8px", color: "#8b98ab" }}>
                  {s.current_stage ?? (s.stages_completed?.length ? "done" : "—")}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {thinking.length > 0 && (
        <div style={{ marginTop: 10 }}>
          <div style={{ color: "#8b98ab", fontSize: 11 }}>
            {thinking.length} raw model call{thinking.length === 1 ? "" : "s"} recorded for this task's thinking
            cycle — no real field ties any of them to a specific step above, so they're listed separately rather
            than guessed into a row:
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: 3, marginTop: 4 }}>
            {thinking.map((t, i) => (
              <div key={i} style={{ fontSize: 11, color: "#c7d0dc" }}>
                <span style={{ color: "#8b98ab" }}>{t.stage}</span> — {t.model_used ?? "unknown model"}
                {t.tokens_used != null ? `, ${t.tokens_used} tok` : ""}
                {t.eval_tokens_per_sec != null ? `, ${t.eval_tokens_per_sec.toFixed(1)} tok/s (BitNet)` : ""}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};

export default StepExecutionList;
