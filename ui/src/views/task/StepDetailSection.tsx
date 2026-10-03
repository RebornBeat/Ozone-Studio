/**
 * H1 — Step detail panel. Pure frontend: `POST /task/get` already returns
 * every field rendered here (verified live against real tasks 10/11/12) —
 * no new backend work.
 *
 * `components/TaskDetailPanel.tsx` already renders, per step: step_index,
 * action, pipeline_id (raw number), status, model_used, tokens_used, error.
 * This section renders exactly what ISN'T already shown there, confirmed
 * present on the real live response and currently invisible anywhere in the
 * UI: the full `output_summary` (today only a truncated preview exists
 * implicitly via thinking_log for some stages — the step's own full output
 * is never shown at all), `started_at`/`completed_at` (real per-step
 * timing/duration), `stages_completed`/`stages_pending`/`current_stage`
 * (real pipeline-stage progress), `graph_ids_read`/`graph_ids_updated`
 * (real graph interaction footprint), `methodology_ids_applied`,
 * `context_assembled`/`context_sources`, and the task-level
 * `blueprint_id`/`blueprint_name`/`description`/`assignee`/`created_by`
 * fields (also returned by /task/get but never displayed).
 *
 * Deliberately NOT rendered here: `version`/`version_notes` per step — that
 * field is real too, but per-step version history is H3's job
 * (VersionNotesSection.tsx), not duplicated here.
 *
 * pipeline_id → human name resolution: `fetchPipelineRegistry()` from
 * ../../ozoneClient (real, already used elsewhere) returns {id,name,...}
 * for ids 1+. Real step data uses pipeline_id 0 ("Process the user prompt"),
 * which has NO registry entry — shown honestly as "no registry entry",
 * never guessed.
 */
import React, { useEffect, useState } from "react";
import { fetchPipelineRegistry, PipelineRegistryEntry } from "../../ozoneClient";

interface StepDetail {
  step_index: number;
  action: string;
  pipeline_id: number;
  status: string;
  started_at?: number | null;
  completed_at?: number | null;
  output_summary?: string | null;
  error?: string | null;
  stages_completed?: string[];
  stages_pending?: string[];
  current_stage?: string | null;
  graph_ids_read?: string[];
  graph_ids_updated?: string[];
  methodology_ids_applied?: number[];
  context_assembled?: unknown;
  context_sources?: string[];
}

interface TaskShape {
  blueprint_id?: number | null;
  blueprint_name?: string | null;
  description?: string | null;
  assignee?: string | null;
  created_by?: string | null;
  steps?: StepDetail[];
}

function fmtTime(ts?: number | null): string {
  if (!ts) return "—";
  // Real timestamps are Unix seconds (confirmed against live task 10/11/12).
  return new Date(ts * 1000).toLocaleString();
}

function fmtDuration(start?: number | null, end?: number | null): string | null {
  if (!start || !end || end < start) return null;
  const s = end - start;
  return s < 1 ? "<1s" : `${s}s`;
}

const Row: React.FC<{ label: string; children: React.ReactNode }> = ({ label, children }) => (
  <div style={{ display: "flex", gap: 6, fontSize: 11.5, marginBottom: 2 }}>
    <span style={{ color: "var(--color-text-muted)", flexShrink: 0 }}>{label}:</span>
    <span style={{ color: "var(--color-text-secondary)", overflowWrap: "anywhere" }}>{children}</span>
  </div>
);

export const StepDetailSection: React.FC<{ task: TaskShape }> = ({ task }) => {
  const steps = task?.steps ?? [];
  const [expanded, setExpanded] = useState<Set<number>>(new Set());
  const [registry, setRegistry] = useState<PipelineRegistryEntry[] | null>(null);
  const [registryError, setRegistryError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    fetchPipelineRegistry()
      .then((r) => {
        if (!cancelled) setRegistry(r.registry ?? []);
      })
      .catch((e) => !cancelled && setRegistryError(e instanceof Error ? e.message : String(e)));
    return () => {
      cancelled = true;
    };
  }, []);

  if (steps.length === 0) return null;

  function pipelineName(id: number): string {
    const entry = registry?.find((r) => r.pipeline_id === id);
    if (entry) return entry.name;
    if (registry) return "no registry entry"; // registry loaded, id genuinely absent (e.g. real id 0)
    return registryError ? "registry unavailable" : "…";
  }

  function toggle(i: number) {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(i)) next.delete(i);
      else next.add(i);
      return next;
    });
  }

  return (
    <div style={{ marginTop: 16 }}>
      <div className="ostat-label" style={{ marginBottom: 8 }}>
        Step detail
      </div>
      {(task.blueprint_name || task.description || task.assignee || task.created_by) && (
        <div style={{ fontSize: 11.5, color: "var(--color-text-muted)", marginBottom: 10, lineHeight: 1.6 }}>
          {task.blueprint_name && <Row label="Blueprint">{task.blueprint_name}{task.blueprint_id != null ? ` (#${task.blueprint_id})` : ""}</Row>}
          {task.assignee && <Row label="Assignee">{task.assignee}</Row>}
          {task.created_by && <Row label="Created by">{task.created_by}</Row>}
        </div>
      )}
      <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
        {steps.map((s) => {
          const isOpen = expanded.has(s.step_index);
          const duration = fmtDuration(s.started_at, s.completed_at);
          return (
            <div key={s.step_index} style={{ border: "1px solid #223046", borderRadius: 8, padding: "8px 10px", fontSize: 12 }}>
              <div
                onClick={() => toggle(s.step_index)}
                style={{ display: "flex", justifyContent: "space-between", gap: 8, cursor: "pointer", opacity: 0.85 }}
              >
                <span>
                  {isOpen ? "▾" : "▸"} Step {s.step_index} — pipeline {s.pipeline_id} ({pipelineName(s.pipeline_id)})
                </span>
                <span style={{ opacity: 0.6, fontSize: 11 }}>{duration ? `${duration}` : ""}</span>
              </div>
              {isOpen && (
                <div style={{ marginTop: 8 }}>
                  <Row label="Started">{fmtTime(s.started_at)}</Row>
                  <Row label="Completed">{fmtTime(s.completed_at)}</Row>
                  {s.current_stage && <Row label="Current stage">{s.current_stage}</Row>}
                  {(s.stages_completed?.length ?? 0) > 0 && (
                    <Row label="Stages completed">{s.stages_completed!.join(" → ")}</Row>
                  )}
                  {(s.stages_pending?.length ?? 0) > 0 && (
                    <Row label="Stages pending">{s.stages_pending!.join(" → ")}</Row>
                  )}
                  {(s.graph_ids_read?.length ?? 0) > 0 && (
                    <Row label="Graphs read">{s.graph_ids_read!.join(", ")}</Row>
                  )}
                  {(s.graph_ids_updated?.length ?? 0) > 0 && (
                    <Row label="Graphs updated">{s.graph_ids_updated!.join(", ")}</Row>
                  )}
                  {(s.methodology_ids_applied?.length ?? 0) > 0 && (
                    <Row label="Methodologies applied">{s.methodology_ids_applied!.join(", ")}</Row>
                  )}
                  {(s.context_sources?.length ?? 0) > 0 && (
                    <Row label="Context sources">{s.context_sources!.join(", ")}</Row>
                  )}
                  {s.context_assembled != null && (
                    <Row label="Context assembled">{typeof s.context_assembled === "string" ? s.context_assembled : JSON.stringify(s.context_assembled)}</Row>
                  )}
                  {s.error && (
                    <div style={{ marginTop: 6, color: "#f87171", fontSize: 11.5, whiteSpace: "pre-wrap" }}>{s.error}</div>
                  )}
                  {s.output_summary ? (
                    <div
                      style={{
                        marginTop: 8,
                        whiteSpace: "pre-wrap",
                        fontSize: 11.5,
                        color: "var(--color-text-secondary)",
                        maxHeight: 260,
                        overflowY: "auto",
                        border: "1px solid var(--color-border-faint)",
                        borderRadius: 6,
                        padding: "6px 8px",
                        background: "var(--color-bg)",
                      }}
                    >
                      {s.output_summary}
                    </div>
                  ) : (
                    <div style={{ marginTop: 6, fontSize: 11, color: "var(--color-text-muted)", fontStyle: "italic" }}>
                      No output recorded for this step.
                    </div>
                  )}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
};

export default StepDetailSection;
