/**
 * H7 — Consciousness gate result per task (decision/confidence/reasoning),
 * from the real `decision_review.jsonl` store (S10) behind `DecisionReviewExecutor`
 * (src/orchestrator/decision_review.rs, pipeline 39's real replacement for the
 * old hardcoded-Evaluate stub — jurisdiction confirmation reviews and the
 * consciousness gate both route through it).
 *
 * NO RELIABLE PER-TASK CORRELATION KEY EXISTS. Verified directly against the
 * real capture site (`DecisionReviewExecutor::capture`, decision_review.rs
 * ~317-345) and the live host (`GET /capture/decision-reviews`): every row
 * carries only `ts`/`model_used`/`tokens_used`/`decision`/`confidence`/
 * `task_summary_preview`/`reasoning_preview`/`raw_response_preview` — no
 * `task_id`, no `project_id`, nothing an id-based join could use. The real
 * `TaskInfo` shape this section receives (`TaskDetailPanel.tsx`) has no
 * comparable free-text field either (`steps[].output_summary`/`error` are
 * per-step execution notes, not the task_summary text that was sent for
 * review) — so even a heuristic substring match has nothing trustworthy to
 * compare against. Per this plan's doctrine (never imply a link that isn't
 * real), this section does NOT attempt to single out "the" review for this
 * task. It states the gap plainly and, for context only, offers the most
 * recent reviews clearly labeled as global activity, not this task's.
 *
 * Real decision/confidence semantics reused from E2/E4's already-verified
 * findings, not re-derived: "proceed"/"decline" are real per-chunk or merged
 * judgments (merged-row confidence is the hardcoded constant 0.8/0.7, not a
 * model score); "chunk-no-judgment" and "review_pending" mean no chunk
 * produced a usable decision; "review-failed" means every review model
 * failed before generating (methodology 43: the failure itself is captured).
 */
import React, { useEffect, useState } from "react";
import { DecisionReviewRow, fetchDecisionReviews } from "../../data/captureData";
import { CaptureStatusBadge, classifyDecisionReview } from "../capture/captureStatus";
import { navigateTo } from "../../navigation";

export type ConsciousnessGateSectionProps = { task: any };

const C_TEXT = "var(--color-text)";
const C_BODY = "var(--color-text-secondary)";
const C_MUTED = "var(--color-text-muted)";
const C_BORDER = "var(--color-border-faint)";
const C_WARN = "#e8c14f";

function decisionWord(d: string): string {
  switch (d) {
    case "proceed":
      return "Proceed";
    case "decline":
      return "Decline";
    case "review_pending":
      return "Review pending";
    case "chunk-no-judgment":
      return "No usable judgment (chunk)";
    case "review-failed":
      return "Review failed";
    default:
      return d;
  }
}

const RECENT_COUNT = 3;

export const ConsciousnessGateSection: React.FC<ConsciousnessGateSectionProps> = () => {
  const [rows, setRows] = useState<DecisionReviewRow[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    fetchDecisionReviews({ limit: RECENT_COUNT })
      .then((page) => {
        if (!cancelled) setRows(page.rows);
      })
      .catch((e) => !cancelled && setError(e instanceof Error ? e.message : String(e)));
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <div>
      <div style={{ fontSize: 12.5, fontWeight: 700, color: C_TEXT, marginBottom: 6 }}>Consciousness gate (pipeline 39)</div>

      <div
        style={{
          border: `1px solid ${C_WARN}`,
          color: C_WARN,
          borderRadius: 6,
          padding: "6px 8px",
          marginBottom: 10,
          fontSize: 11.5,
          lineHeight: 1.5,
        }}
      >
        The real decision-review capture store records no task id or project id — only a free-text
        summary preview of what was reviewed. There is no reliable way to show "the" gate result for
        this specific task, and this section does not guess one. The rows below are the most recent
        gate reviews system-wide, for context only — they are <b>not</b> known to belong to this task.
      </div>

      {error && <div style={{ fontSize: 12, color: "#ff8a8a" }}>Error loading recent reviews: {error}</div>}
      {!error && rows === null && <div style={{ fontSize: 12, color: C_MUTED }}>Loading recent reviews…</div>}
      {!error && rows !== null && rows.length === 0 && (
        <div style={{ fontSize: 12, color: C_MUTED }}>No decision reviews have been captured yet.</div>
      )}

      {rows && rows.length > 0 && (
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {rows.map((r, i) => {
            const status = classifyDecisionReview(r);
            return (
              <div key={i} style={{ border: `1px solid ${C_BORDER}`, borderRadius: 8, padding: "8px 10px", fontSize: 12 }}>
                <div style={{ display: "flex", justifyContent: "space-between", gap: 8, marginBottom: 4 }}>
                  <span style={{ color: C_TEXT, fontWeight: 600 }}>{decisionWord(r.decision)}</span>
                  <CaptureStatusBadge status={status} />
                </div>
                <div style={{ color: C_MUTED, marginBottom: 4 }}>
                  {r.ts} · {r.model_used}
                  {r.confidence !== null && ` · confidence ${r.confidence.toFixed(2)}`}
                </div>
                <div style={{ color: C_BODY, whiteSpace: "pre-wrap" }}>{r.reasoning_preview}</div>
              </div>
            );
          })}
        </div>
      )}

      <button
        onClick={() => navigateTo({ kind: "tab", tabId: "capture-viewer" })}
        style={{
          marginTop: 10,
          background: "transparent",
          border: `1px solid ${C_BORDER}`,
          color: C_BODY,
          borderRadius: 6,
          padding: "4px 10px",
          fontSize: 11.5,
          cursor: "pointer",
        }}
      >
        Open full Decision Review browser
      </button>
    </div>
  );
};

export default ConsciousnessGateSection;
