/**
 * E2 — Decision Review panel (S10 `decision_review.jsonl`).
 *
 * Real sources:
 *  - Rows: GET /capture/decision-reviews via `fetchDecisionReviews` (data/captureData.ts, B4). 8 fields per
 *    row; `raw_response_preview` is nullable because older lines predate it. The response has NO total, so
 *    paging is "load more until a short page".
 *  - Decision semantics: src/orchestrator/decision_review.rs. The values the CURRENT source writes are
 *    "proceed" | "decline" (per chunk and merged), "chunk-no-judgment" (a chunk's response had no usable
 *    decision), "review_pending" (no chunk produced a judgment — never a default Proceed) and
 *    "review-failed" (every review model failed before generating; methodology 43: failures leave records).
 *    Merge rule: ANY chunk decline => overall decline; the merged row's confidence is the constants 0.7
 *    (decline) / 0.8 (proceed) set in code, not a model-reported number.
 *  - capture() cuts task_summary/reasoning/raw_response to 500 characters, so previews may be truncated.
 *  - The store is a flat append-only file: it emits no graph events, so there is no live push — use Reload.
 *    It has no project key, so `projectId` cannot filter anything (stated in the UI, not faked).
 * Failure is the primary state: every row currently on disk is a failed/unresolved review.
 */
import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { DecisionReviewRow, fetchDecisionReviews } from "../../data/captureData";
import { CaptureSeverity, CaptureStatusBadge, classifyDecisionReview } from "./captureStatus";

export type DecisionReviewPanelProps = { projectId: number | null };

const PAGE = 100;

const C_TEXT = "var(--color-text)";
const C_BODY = "var(--color-text-secondary)";
const C_MUTED = "var(--color-text-muted)";
const C_BORDER = "var(--color-border-faint)";
const C_PANEL = "var(--color-bg)";
const C_FAIL = "#ff8a8a";
const C_WARN = "#e8c14f";
const C_OK = "#8fe38f";
const C_VERDICT = "#ffb95f";

type Tone = "failure" | "pending" | "proceed" | "decline" | "unknown";

interface DecisionInfo {
  label: string;
  tone: Tone;
  explain: string;
}

function decisionInfo(decision: string): DecisionInfo {
  switch (decision) {
    case "proceed":
      return {
        label: "proceed",
        tone: "proceed",
        explain:
          "A review model judged the task raises no concern. Overall Proceed requires that no chunk declined and at least one chunk produced a valid judgment. On the merged row the confidence is the constant 0.8 set in code, not a model-reported value.",
      };
    case "decline":
      return {
        label: "decline",
        tone: "decline",
        explain:
          "A review model judged the task should be declined. Safety-first merge: any Decline across chunks makes the overall result Decline. On the merged row the confidence is the constant 0.7 set in code, not a model-reported value.",
      };
    case "review_pending":
      return {
        label: "review_pending",
        tone: "pending",
        explain:
          "No chunk produced a valid judgment (placeholders / unparseable output), so the result is ReviewPending — needs human review. It is deliberately never a default Proceed.",
      };
    case "chunk-no-judgment":
      return {
        label: "chunk-no-judgment",
        tone: "failure",
        explain:
          "This chunk's model response contained no usable decision (no parseable JSON with a recognised decision and non-placeholder reasoning). Recorded so the absence of judgment is itself visible.",
      };
    case "review-failed":
      return {
        label: "review-failed",
        tone: "failure",
        explain:
          "The review errored before any judgment — typically every review model in the fallback chain failed or returned an unusable response. Captured deliberately (methodology 43: failures leave records); the gate then reports ReviewPending so a human decides.",
      };
    default:
      return {
        label: decision || "(empty)",
        tone: "unknown",
        explain:
          "Not one of the decision values the current decision_review.rs writes (proceed, decline, review_pending, chunk-no-judgment, review-failed). Shown verbatim.",
      };
  }
}

const TONE_COLOR: Record<Tone, string> = {
  failure: C_FAIL,
  pending: C_WARN,
  proceed: C_OK,
  decline: C_VERDICT,
  unknown: C_MUTED,
};
const SEVERITY_COLOR: Record<Exclude<CaptureSeverity, "unclassified">, string> = {
  success: C_OK,
  fallback: C_WARN,
  failure: C_FAIL,
  "review-pending": C_WARN,
};

/** Per-chunk rows carry a "[chunk i/n]" suffix on the task summary; the merged verdict row does not. */
function parseChunk(summary: string): { index: number; total: number } | null {
  const m = summary.match(/\[chunk (\d+)\/(\d+)\]\s*$/);
  return m ? { index: Number(m[1]), total: Number(m[2]) } : null;
}

/** Explains the reason strings whose origin is verifiable in the current source; never guesses the rest. */
function reasonNote(reason: string): string | null {
  if (/^chunk \d+\/\d+: all review models returned unusable responses$/.test(reason)) {
    return "Raised by decision_review.rs after the primary model AND the fallback chain all returned empty/failed responses for this chunk.";
  }
  if (/^(invalid decision value|decision field missing|no JSON object with a decision|candidate missing decision)/.test(reason)) {
    return "This wording is not produced by the current decision_review.rs — it was written by an earlier review implementation. Shown verbatim.";
  }
  return null;
}

interface Decorated {
  row: DecisionReviewRow;
  index: number;
  info: DecisionInfo;
  severity: CaptureSeverity;
  chunk: { index: number; total: number } | null;
}

function colorFor(d: Decorated): string {
  return d.severity !== "unclassified" ? SEVERITY_COLOR[d.severity] : TONE_COLOR[d.info.tone];
}
function isFailure(d: Decorated): boolean {
  return d.severity === "unclassified" ? d.info.tone === "failure" : d.severity === "failure";
}

const selectStyle: React.CSSProperties = {
  background: "#101724",
  color: C_TEXT,
  border: `1px solid ${C_BORDER}`,
  borderRadius: 6,
  padding: "3px 8px",
  fontSize: 12,
  maxWidth: 260,
};
const preStyle: React.CSSProperties = {
  margin: "4px 0 0",
  background: C_PANEL,
  border: `1px solid ${C_BORDER}`,
  borderRadius: 6,
  padding: "6px 8px",
  fontSize: 11.5,
  color: C_BODY,
  whiteSpace: "pre-wrap",
  overflowWrap: "anywhere",
  maxHeight: 200,
  overflowY: "auto",
  fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
};

const Field: React.FC<{ label: string; text: string; note?: string }> = ({ label, text, note }) => (
  <div style={{ marginTop: 8 }}>
    <div style={{ fontSize: 11, fontWeight: 700, letterSpacing: 0.4, textTransform: "uppercase", color: C_MUTED }}>
      {label}
    </div>
    <pre style={preStyle}>{text}</pre>
    {note && <div style={{ fontSize: 11, color: C_MUTED, marginTop: 2 }}>{note}</div>}
  </div>
);

const CUT_NOTE = "Preview is cut to 500 characters when captured (decision_review.rs capture()).";

function RawResponse({ raw }: { raw: string | null }) {
  if (raw === null) {
    return (
      <div style={{ marginTop: 8, fontSize: 12, color: C_MUTED, fontStyle: "italic" }}>
        Raw response: not captured on this older line (the raw_response_preview field predates it).
      </div>
    );
  }
  if (raw === "") {
    return (
      <div style={{ marginTop: 8, fontSize: 12, color: C_MUTED, fontStyle: "italic" }}>
        Raw response: empty — nothing recorded (the merged final-verdict row is written without one).
      </div>
    );
  }
  return <Field label="Raw response" text={raw} note={raw.length >= 500 ? CUT_NOTE : undefined} />;
}

export const DecisionReviewPanel: React.FC<DecisionReviewPanelProps> = ({ projectId }) => {
  const [rows, setRows] = useState<DecisionReviewRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [hasMore, setHasMore] = useState(false);
  const [decisionFilter, setDecisionFilter] = useState("all");
  const [modelFilter, setModelFilter] = useState("all");
  const [newestFirst, setNewestFirst] = useState(true);
  const [expanded, setExpanded] = useState<Set<number>>(new Set());
  const nextOffset = useRef(0);
  const inFlight = useRef(false);

  const loadPage = useCallback(async (reset: boolean) => {
    if (inFlight.current) return;
    inFlight.current = true;
    if (reset) nextOffset.current = 0;
    setLoading(true);
    setError(null);
    try {
      const page = await fetchDecisionReviews({ offset: nextOffset.current, limit: PAGE });
      nextOffset.current += page.rows.length;
      setRows((prev) => (reset ? page.rows : [...prev, ...page.rows]));
      // No `total` in the response: a full page means there may be more, a short page means we hit the end.
      setHasMore(page.rows.length >= PAGE);
      if (reset) setExpanded(new Set());
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      inFlight.current = false;
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadPage(true);
  }, [loadPage]);

  const decorated = useMemo<Decorated[]>(
    () =>
      rows.map((row, index) => ({
        row,
        index,
        info: decisionInfo(row.decision),
        severity: classifyDecisionReview(row).severity,
        chunk: parseChunk(row.task_summary_preview),
      })),
    [rows],
  );

  const decisionOptions = useMemo(() => Array.from(new Set(rows.map((r) => r.decision))).sort(), [rows]);
  const modelOptions = useMemo(() => Array.from(new Set(rows.map((r) => r.model_used))).sort(), [rows]);
  const decisionCounts = useMemo(() => {
    const m = new Map<string, number>();
    for (const r of rows) m.set(r.decision, (m.get(r.decision) ?? 0) + 1);
    return Array.from(m.entries()).sort((a, b) => b[1] - a[1]);
  }, [rows]);
  const failedCount = useMemo(() => decorated.filter(isFailure).length, [decorated]);

  const visible = useMemo(() => {
    const filtered = decorated.filter(
      (d) => (decisionFilter === "all" || d.row.decision === decisionFilter) && (modelFilter === "all" || d.row.model_used === modelFilter),
    );
    return newestFirst ? [...filtered].reverse() : filtered;
  }, [decorated, decisionFilter, modelFilter, newestFirst]);

  const toggle = (i: number) =>
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(i)) next.delete(i);
      else next.add(i);
      return next;
    });

  return (
    <div style={{ color: C_BODY, fontSize: 12.5 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 10, flexWrap: "wrap", marginBottom: 6 }}>
        <span style={{ fontSize: 13, fontWeight: 700, color: C_TEXT }}>Decision reviews</span>
        <button
          onClick={() => loadPage(true)}
          disabled={loading}
          style={{ ...selectStyle, cursor: loading ? "default" : "pointer" }}
          title="Re-read the capture file — this store emits no live events"
        >
          {loading ? "Loading…" : "Reload"}
        </button>
      </div>
      <div style={{ fontSize: 11.5, color: C_MUTED, marginBottom: 10, lineHeight: 1.5 }}>
        Every consciousness/jurisdiction decision review exactly as captured (decision_review.jsonl).
        {projectId !== null &&
          ` Reviews are not stored per project, so all of them are shown (project ${projectId} cannot filter this list).`}{" "}
        Filters apply to the rows loaded so far; the route has no server-side filter.
      </div>

      {error && (
        <div style={{ border: `1px solid ${C_FAIL}`, color: C_FAIL, borderRadius: 6, padding: "6px 8px", marginBottom: 10 }}>
          Could not load decision reviews: {error}
        </div>
      )}

      {!error && !loading && rows.length === 0 && (
        <div style={{ color: C_MUTED, padding: "12px 0" }}>
          No decision reviews have been captured yet — nothing to show (nothing is fabricated in its place).
        </div>
      )}
      {!error && loading && rows.length === 0 && <div style={{ color: C_MUTED, padding: "12px 0" }}>Loading decision reviews…</div>}

      {rows.length > 0 && (
        <>
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center", marginBottom: 10 }}>
            <span style={{ color: failedCount > 0 ? C_FAIL : C_MUTED, fontWeight: 600 }}>
              {failedCount} of {rows.length} loaded row{rows.length === 1 ? "" : "s"} failed / no judgment
            </span>
            {decisionCounts.map(([d, n]) => (
              <span
                key={d}
                title={decisionInfo(d).explain}
                style={{
                  border: `1px solid ${TONE_COLOR[decisionInfo(d).tone]}`,
                  color: TONE_COLOR[decisionInfo(d).tone],
                  borderRadius: 999,
                  padding: "1px 8px",
                  fontSize: 11.5,
                  cursor: "help",
                }}
              >
                {decisionInfo(d).label} · {n}
              </span>
            ))}
          </div>

          <div style={{ display: "flex", gap: 10, flexWrap: "wrap", alignItems: "center", marginBottom: 10 }}>
            <label style={{ color: C_MUTED }}>
              Decision{" "}
              <select value={decisionFilter} onChange={(e) => setDecisionFilter(e.target.value)} style={selectStyle}>
                <option value="all">all</option>
                {decisionOptions.map((d) => (
                  <option key={d} value={d}>
                    {d || "(empty)"}
                  </option>
                ))}
              </select>
            </label>
            <label style={{ color: C_MUTED }}>
              Model{" "}
              <select value={modelFilter} onChange={(e) => setModelFilter(e.target.value)} style={selectStyle}>
                <option value="all">all</option>
                {modelOptions.map((m) => (
                  <option key={m} value={m}>
                    {m}
                  </option>
                ))}
              </select>
            </label>
            <label style={{ color: C_MUTED, display: "flex", alignItems: "center", gap: 4, cursor: "pointer" }}>
              <input type="checkbox" checked={newestFirst} onChange={() => setNewestFirst((v) => !v)} style={{ margin: 0 }} />
              Newest first
            </label>
            <span style={{ color: C_MUTED, fontSize: 11.5 }}>
              showing {visible.length} of {rows.length}
            </span>
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            {visible.map((d) => {
              const { row } = d;
              const open = expanded.has(d.index);
              const color = colorFor(d);
              const status = classifyDecisionReview(row);
              const note = reasonNote(row.reasoning_preview);
              return (
                <div
                  key={d.index}
                  style={{ border: `1px solid ${C_BORDER}`, borderLeft: `3px solid ${color}`, borderRadius: 8, padding: "7px 10px", background: "rgba(255,255,255,0.015)" }}
                >
                  <div
                    onClick={() => toggle(d.index)}
                    style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap", cursor: "pointer" }}
                  >
                    <span style={{ color: C_MUTED }}>{open ? "▾" : "▸"}</span>
                    <span style={{ color: C_MUTED, fontSize: 11.5 }}>{row.ts}</span>
                    <span
                      title={d.info.explain}
                      style={{ color, border: `1px solid ${color}`, borderRadius: 999, padding: "0 8px", fontSize: 11.5, fontWeight: 600, cursor: "help" }}
                    >
                      {d.info.label}
                    </span>
                    <CaptureStatusBadge status={status} />
                    {d.chunk ? (
                      <span style={{ color: C_MUTED, fontSize: 11.5 }}>
                        chunk {d.chunk.index}/{d.chunk.total}
                      </span>
                    ) : (
                      <span style={{ color: C_MUTED, fontSize: 11.5 }} title="Rows without a [chunk i/n] suffix are the merged final verdict, or a review that failed before chunking">
                        whole review
                      </span>
                    )}
                    <span style={{ color: C_MUTED, fontSize: 11.5 }}>
                      confidence: {row.confidence === null ? "none recorded" : row.confidence.toFixed(2)}
                    </span>
                    <span style={{ color: C_MUTED, fontSize: 11.5 }}>{row.tokens_used} tok</span>
                    <span
                      style={{ color: row.model_used === "none (all models failed)" ? C_FAIL : C_BODY, fontSize: 11.5, overflowWrap: "anywhere" }}
                    >
                      model: {row.model_used}
                    </span>
                  </div>
                  {!open && (
                    <div style={{ marginTop: 4, marginLeft: 18, color: isFailure(d) ? C_FAIL : C_BODY, fontSize: 12, overflowWrap: "anywhere" }}>
                      {isFailure(d) ? "Why: " : ""}
                      {row.reasoning_preview || <span style={{ color: C_MUTED, fontStyle: "italic" }}>no reasoning recorded</span>}
                    </div>
                  )}
                  {open && (
                    <div style={{ marginLeft: 18 }}>
                      <div style={{ marginTop: 6, fontSize: 12, color: C_BODY, lineHeight: 1.5 }}>{d.info.explain}</div>
                      {status.reasons.length > 0 && status.severity !== "unclassified" && (
                        <div style={{ marginTop: 4, fontSize: 11.5, color: C_MUTED }}>Status: {status.reasons.join("; ")}</div>
                      )}
                      <Field
                        label="Task summary"
                        text={row.task_summary_preview || "(empty)"}
                        note={row.task_summary_preview.length >= 500 ? CUT_NOTE : undefined}
                      />
                      <Field
                        label="Reasoning"
                        text={row.reasoning_preview || "(no reasoning recorded)"}
                        note={note ?? (row.reasoning_preview.length >= 500 ? CUT_NOTE : undefined)}
                      />
                      <RawResponse raw={row.raw_response_preview} />
                    </div>
                  )}
                </div>
              );
            })}
            {visible.length === 0 && <div style={{ color: C_MUTED, padding: "8px 0" }}>No loaded rows match these filters.</div>}
          </div>

          <div style={{ marginTop: 12, display: "flex", gap: 10, alignItems: "center", flexWrap: "wrap" }}>
            {hasMore && (
              <button onClick={() => loadPage(false)} disabled={loading} style={{ ...selectStyle, cursor: loading ? "default" : "pointer" }}>
                {loading ? "Loading…" : `Load ${PAGE} more`}
              </button>
            )}
            <span style={{ color: C_MUTED, fontSize: 11.5 }}>
              {hasMore
                ? "The file is stored oldest-first, so newer rows arrive as you load more."
                : "End of the capture file — every stored row is loaded."}
            </span>
          </div>
        </>
      )}
    </div>
  );
};

export default DecisionReviewPanel;
