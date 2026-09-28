/**
 * E6 — Confetti-burst raw-response viewer.
 *
 * WHAT A "CONFETTI BURST" REALLY IS (repo definition, not invented here):
 * one model call whose single response contains MORE THAN ONE distinct,
 * non-empty, parseable JSON candidate — the BitNet failure mode found live
 * 2026-09-22, e.g. `Decide 0.9 → Reject 0.8 → Proceed 0.7 → Fail 0.6 →
 * Accept 0.5` (confidence descending, interleaved with prose, a leading bare
 * `{}`, code fences). It is a generation-quality problem, not a parsing
 * problem (docs/TOP_DOWN_REVIEW_GUIDE.md §3.6; docs/ZERO_SHOT_EXPANSION_GUIDE.md
 * C6 — "true confetti ... self-contradicting inside a single unterminated
 * completion"). Methodology 35 (assets/methodologies/method_35_*) is the
 * related balanced-brace extraction rule.
 *
 * DETECTION RULES — ported 1:1 from the two real implementations:
 *  - S10 decision reviews: `is_confetti()` in src/orchestrator/decision_review.rs
 *    (line 62) = `extract_all_json_objects(response).len() > 1`. Balanced-brace,
 *    string-aware scan from EVERY `{`; a candidate counts only if it balances,
 *    parses, and is a NON-EMPTY object (bare `{}` is skipped). Objects only.
 *  - S11 zero-shot calls: `is_unusable_pipeline9_result()` →
 *    `extract_all_json_candidates_shared()` in src/orchestrator/mod.rs — same
 *    scan, but objects AND arrays; applies only when the response is non-blank.
 * Faithfulness notes (real behaviour, surfaced rather than hidden):
 *  - The scan starts at every `{`, so an object containing a non-empty NESTED
 *    object yields 2 candidates. The real rule counts that as confetti; each
 *    candidate here carries `nestedIn` so the UI can say when a "burst" is
 *    really one object with a nested one.
 *  - Browser `JSON.parse` stands in for `serde_json::from_str`.
 *
 * WHAT IS STORED (so what this viewer can and cannot show):
 *  - S10 `raw_response_preview` is `cut(raw_response, 500)` (decision_review.rs
 *    capture()) — the FIRST 500 CHARACTERS, not the full response. S11
 *    `response_preview` is 500 chars for an Ok response and 300 for an Err
 *    (capture_zero_shot_call, src/orchestrator/mod.rs). A burst whose later
 *    candidates fall past the cap, or a candidate cut mid-object, cannot be
 *    counted — detection here can only UNDER-count, never over-count.
 *  - Some S10 lines hold an executor PLACEHOLDER instead of model output
 *    ("(no raw response — review failed before generation)", decision_review.rs
 *    :375; "(empty or failed response — no raw content to capture)" appears
 *    only on older stored lines). Those are labelled, never shown as model text.
 *  - S10 rows carry no project id; S11 rows do (`project_id`).
 *
 * Data: `fetchDecisionReviews` / `fetchZeroShotCalls` (B4/B5). Read-only. No
 * mock data: empty and error states are shown as such.
 */
import React, { useCallback, useEffect, useMemo, useState } from "react";
import { fetchDecisionReviews, fetchZeroShotCalls } from "../../data/captureData";
import type { CapturePage, DecisionReviewRow, ZeroShotCallRow } from "../../data/captureData";
import { CaptureStatusBadge, classifyDecisionReview, classifyZeroShotCall } from "./captureStatus";

export type ConfettiViewerProps = { projectId: number | null };

// ─────────────────────────────────────────────────────────────────────────
// Real detection rules (pure; exported so they can be exercised directly)
// ─────────────────────────────────────────────────────────────────────────

export interface JsonCandidate {
  /** 1-based, in text order. */
  index: number;
  start: number;
  end: number;
  shape: "object" | "array";
  text: string;
  parsed: unknown;
  /** `index` of the smallest OTHER candidate that fully encloses this one, if any. */
  nestedIn: number | null;
}

/** Balanced, string-aware span scan from every `open` — port of the Rust scan. */
function scanBalanced(s: string, open: string, close: string): Array<[number, number]> {
  const spans: Array<[number, number]> = [];
  for (let start = 0; start < s.length; start++) {
    if (s[start] !== open) continue;
    let depth = 0;
    let inString = false;
    let escaped = false;
    let end = -1;
    for (let i = start; i < s.length; i++) {
      const c = s[i];
      if (inString) {
        if (escaped) escaped = false;
        else if (c === "\\") escaped = true;
        else if (c === '"') inString = false;
        continue;
      }
      if (c === '"') inString = true;
      else if (c === open) depth++;
      else if (c === close) {
        depth--;
        if (depth === 0) {
          end = i;
          break;
        }
      }
    }
    if (end >= 0) spans.push([start, end]);
  }
  return spans;
}

/** Every non-empty parseable JSON candidate, in text order. `includeArrays` = the S11 rule. */
export function extractJsonCandidates(text: string, includeArrays: boolean): JsonCandidate[] {
  const found: Array<Omit<JsonCandidate, "index" | "nestedIn">> = [];
  for (const [start, end] of scanBalanced(text, "{", "}")) {
    const t = text.slice(start, end + 1);
    try {
      const p: unknown = JSON.parse(t);
      if (p !== null && typeof p === "object" && !Array.isArray(p) && Object.keys(p as object).length > 0) {
        found.push({ start, end, shape: "object", text: t, parsed: p });
      }
    } catch {
      /* not JSON — not a candidate */
    }
  }
  if (includeArrays) {
    for (const [start, end] of scanBalanced(text, "[", "]")) {
      const t = text.slice(start, end + 1);
      try {
        const p: unknown = JSON.parse(t);
        if (Array.isArray(p) && p.length > 0) found.push({ start, end, shape: "array", text: t, parsed: p });
      } catch {
        /* not JSON */
      }
    }
  }
  found.sort((a, b) => a.start - b.start || b.end - a.end);
  return found.map((c, i) => {
    let parent: (typeof found)[number] | null = null;
    for (const o of found) {
      if (o === c) continue;
      if (o.start <= c.start && c.end <= o.end && (o.start !== c.start || o.end !== c.end)) {
        if (!parent || o.end - o.start < parent.end - parent.start) parent = o;
      }
    }
    return { ...c, index: i + 1, nestedIn: parent ? found.indexOf(parent) + 1 : null };
  });
}

/** S10 rule — decision_review.rs `is_confetti`: >1 non-empty object candidate. */
export function isConfettiS10(text: string): boolean {
  return extractJsonCandidates(text, false).length > 1;
}
/** S11 rule — mod.rs `is_unusable_pipeline9_result`: non-blank AND >1 object-or-array candidate. */
export function isConfettiS11(text: string): boolean {
  return text.trim() !== "" && extractJsonCandidates(text, true).length > 1;
}

/** Placeholder text the executor writes when there is NO model output to capture. */
const CAPTURE_PLACEHOLDER = /^\((no raw response|empty or failed response)[^)]*\)$/;
export function isCapturePlaceholder(raw: string): boolean {
  return CAPTURE_PLACEHOLDER.test(raw.trim());
}

export interface RawSegment {
  kind: "prose" | "candidate";
  text: string;
  candidateIndex?: number;
}
/** Split raw text into prose/candidate segments using TOP-LEVEL candidates only (no overlaps). */
export function segmentRaw(raw: string, candidates: JsonCandidate[]): RawSegment[] {
  const segments: RawSegment[] = [];
  let pos = 0;
  for (const c of candidates) {
    if (c.nestedIn !== null || c.start < pos) continue;
    if (c.start > pos) segments.push({ kind: "prose", text: raw.slice(pos, c.start) });
    segments.push({ kind: "candidate", text: raw.slice(c.start, c.end + 1), candidateIndex: c.index });
    pos = c.end + 1;
  }
  if (pos < raw.length) segments.push({ kind: "prose", text: raw.slice(pos) });
  return segments;
}

// ─────────────────────────────────────────────────────────────────────────
// Row analysis
// ─────────────────────────────────────────────────────────────────────────

type Analyzed =
  | { source: "S10"; ts: string; row: DecisionReviewRow; raw: string; placeholder: boolean; candidates: JsonCandidate[]; burst: boolean }
  | { source: "S11"; ts: string; row: ZeroShotCallRow; raw: string; placeholder: boolean; candidates: JsonCandidate[]; burst: boolean };

function analyzeDecision(row: DecisionReviewRow): Analyzed {
  const raw = row.raw_response_preview ?? "";
  const placeholder = isCapturePlaceholder(raw);
  const candidates = placeholder ? [] : extractJsonCandidates(raw, false);
  return { source: "S10", ts: row.ts, row, raw, placeholder, candidates, burst: candidates.length > 1 };
}
function analyzeCall(row: ZeroShotCallRow): Analyzed {
  const raw = row.response_preview ?? "";
  const candidates = extractJsonCandidates(raw, true);
  return { source: "S11", ts: row.ts, row, raw, placeholder: false, candidates, burst: raw.trim() !== "" && candidates.length > 1 };
}

const isJudgment = (d: string) => ["proceed", "decline"].includes(d.trim().toLowerCase());
/** "Rejected / failed" for the secondary list: no usable judgment (S10) or an unsuccessful call (S11). */
function isRejectedOrFailed(a: Analyzed): boolean {
  return a.source === "S10" ? !isJudgment(a.row.decision) : !a.row.success;
}

// ─────────────────────────────────────────────────────────────────────────
// Fetching (all pages; the routes cap a page at 500 and carry no total)
// ─────────────────────────────────────────────────────────────────────────

const PAGE = 500;
const MAX_PAGES = 20;

async function fetchAllPages<T>(
  fetchPage: (o: { offset: number; limit: number }) => Promise<CapturePage<T>>,
): Promise<{ rows: T[]; capped: boolean }> {
  const rows: T[] = [];
  for (let page = 0; page < MAX_PAGES; page++) {
    const res = await fetchPage({ offset: page * PAGE, limit: PAGE });
    rows.push(...res.rows);
    if (res.rows.length < PAGE) return { rows, capped: false };
  }
  return { rows, capped: true };
}

interface SourceState<T> {
  rows: T[];
  error: string | null;
  capped: boolean;
}
const emptySource = <T,>(): SourceState<T> => ({ rows: [], error: null, capped: false });

// ─────────────────────────────────────────────────────────────────────────
// Presentation
// ─────────────────────────────────────────────────────────────────────────

const C_TEXT = "#dfe7f2";
const C_BODY = "#c7d0dc";
const C_MUTED = "#8b98ab";
const C_BORDER = "#1e2836";
const C_PANEL = "#0a0f1a";
const C_WARN = "#e8c14f";
const C_BAD = "#ff8a8a";
const CANDIDATE_COLORS = ["#5fb3ff", "#ffb95f", "#8fe38f", "#d68fe3", "#ff8a8a", "#7fe0d0"];

const wrap: React.CSSProperties = { overflowWrap: "anywhere", wordBreak: "break-word", minWidth: 0 };
const card: React.CSSProperties = {
  border: `1px solid ${C_BORDER}`,
  borderRadius: 10,
  background: C_PANEL,
  padding: "10px 12px",
  marginBottom: 10,
};
const preBase: React.CSSProperties = {
  ...wrap,
  margin: "6px 0 0",
  padding: "8px 10px",
  background: "#070b13",
  border: `1px solid ${C_BORDER}`,
  borderRadius: 6,
  fontSize: 11.5,
  lineHeight: 1.5,
  color: C_BODY,
  whiteSpace: "pre-wrap",
  maxHeight: 260,
  overflowY: "auto",
  fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
};

const chip = (color: string): React.CSSProperties => ({
  fontSize: 10.5,
  fontWeight: 700,
  letterSpacing: 0.3,
  color,
  border: `1px solid ${color}`,
  borderRadius: 999,
  padding: "1px 7px",
  whiteSpace: "nowrap",
});

function fmtTs(ts: string): string {
  const d = new Date(ts);
  return Number.isNaN(d.getTime()) ? ts : d.toLocaleString();
}

const Row: React.FC<{ label: string; children: React.ReactNode }> = ({ label, children }) => (
  <div style={{ ...wrap, marginBottom: 2, fontSize: 12 }}>
    <span style={{ color: C_MUTED }}>{label}: </span>
    <span style={{ color: C_BODY }}>{children}</span>
  </div>
);

const CardHeader: React.FC<{ a: Analyzed; extra?: React.ReactNode }> = ({ a, extra }) => (
  <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap", marginBottom: 6 }}>
    <span style={chip(a.source === "S10" ? "#5fb3ff" : "#ffb95f")}>{a.source === "S10" ? "S10 decision review" : "S11 model call"}</span>
    <span style={{ fontSize: 11.5, color: C_MUTED }}>{fmtTs(a.ts)}</span>
    {a.source === "S10" ? (
      <CaptureStatusBadge status={classifyDecisionReview(a.row)} />
    ) : (
      <CaptureStatusBadge status={classifyZeroShotCall(a.row)} />
    )}
    {extra}
  </div>
);

/** What the gate recorded for the row, plus the model. */
const GateSummary: React.FC<{ a: Analyzed }> = ({ a }) =>
  a.source === "S10" ? (
    <>
      <Row label="Model">{a.row.model_used}</Row>
      <Row label="Gate decision">
        <b style={{ color: isJudgment(a.row.decision) ? C_TEXT : C_BAD }}>{a.row.decision}</b>
        {typeof a.row.confidence === "number" && ` (confidence ${a.row.confidence.toFixed(2)})`}
      </Row>
      <Row label="Gate reasoning">{a.row.reasoning_preview || <i style={{ color: C_MUTED }}>none recorded</i>}</Row>
      <Row label="Task">{a.row.task_summary_preview}</Row>
    </>
  ) : (
    <>
      <Row label="Call site">{a.row.call_site}</Row>
      <Row label="Model">{a.row.model_used || <i style={{ color: C_MUTED }}>no model answered</i>}</Row>
      <Row label="Outcome">
        <b style={{ color: a.row.success ? C_TEXT : C_BAD }}>{a.row.success ? "usable response" : "unusable / failed"}</b>
        {` · ${a.row.retry_count} retr${a.row.retry_count === 1 ? "y" : "ies"}`}
        {a.row.used_fallback && " · fallback walk used"}
        {a.row.tokens_used > 0 && ` · ${a.row.tokens_used} tokens`}
      </Row>
      <Row label="Prompt">{a.row.prompt_preview}</Row>
    </>
  );

function capNote(a: Analyzed): string {
  return a.source === "S10"
    ? "Stored preview: first 500 characters only (decision_review.rs capture())."
    : `Stored preview: first ${a.row.success ? 500 : 300} characters only (capture_zero_shot_call).`;
}

const Candidates: React.FC<{ a: Analyzed }> = ({ a }) => {
  const reasoningCommitted =
    a.source === "S10" ? a.row.reasoning_preview.trim() : null;
  return (
    <div style={{ marginTop: 8 }}>
      <div style={{ fontSize: 11, fontWeight: 700, color: C_MUTED, textTransform: "uppercase", letterSpacing: 0.4, marginBottom: 4 }}>
        {a.candidates.length} candidates parsed from the stored text
      </div>
      {a.candidates.map((c) => {
        const p = c.shape === "object" ? (c.parsed as Record<string, unknown>) : null;
        const decision = p && typeof p.decision === "string" ? p.decision : null;
        const confidence = p && typeof p.confidence === "number" ? p.confidence : null;
        const reasoning = p && typeof p.reasoning === "string" ? p.reasoning : null;
        const committed =
          reasoningCommitted !== null && reasoning !== null && reasoning.trim().slice(0, 500) === reasoningCommitted;
        const color = CANDIDATE_COLORS[(c.index - 1) % CANDIDATE_COLORS.length];
        return (
          <div key={c.index} style={{ ...wrap, borderLeft: `3px solid ${color}`, padding: "2px 8px", marginBottom: 4, fontSize: 11.5, color: C_BODY }}>
            <b style={{ color }}>#{c.index}</b> {c.shape}
            {c.nestedIn !== null && <span style={{ color: C_WARN }}> · nested inside #{c.nestedIn}</span>}
            {decision !== null && (
              <>
                {" · decision "}
                <b>{JSON.stringify(decision)}</b>
              </>
            )}
            {confidence !== null && ` · confidence ${confidence}`}
            {reasoning !== null && <div style={{ color: C_MUTED }}>reasoning: {reasoning || "(empty)"}</div>}
            {decision === null && reasoning === null && (
              <div style={{ color: C_MUTED }}>{c.text.length > 140 ? `${c.text.slice(0, 140)}…` : c.text}</div>
            )}
            {committed && (
              <div style={{ color: "#8fe38f" }}>
                ✓ this candidate's reasoning is exactly what the gate recorded (exact string match) — the candidate the gate committed to
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
};

const RawText: React.FC<{ a: Analyzed; segmented: boolean }> = ({ a, segmented }) => {
  if (a.placeholder) {
    return (
      <div style={{ ...wrap, marginTop: 6, fontSize: 11.5, color: C_MUTED, fontStyle: "italic" }}>
        No model output was captured for this line — the executor stored a placeholder instead ({JSON.stringify(a.raw)}).
      </div>
    );
  }
  if (a.raw.trim() === "") {
    return (
      <div style={{ marginTop: 6, fontSize: 11.5, color: C_MUTED, fontStyle: "italic" }}>
        No raw response stored on this line{a.source === "S10" ? " (older lines predate raw_response_preview)" : ""}.
      </div>
    );
  }
  const segments = segmented ? segmentRaw(a.raw, a.candidates) : null;
  return (
    <>
      <pre style={preBase}>
        {segments
          ? segments.map((s, i) =>
              s.kind === "prose" ? (
                <span key={i} style={{ color: C_MUTED }}>
                  {s.text}
                </span>
              ) : (
                <span
                  key={i}
                  style={{
                    color: C_TEXT,
                    background: "rgba(255,255,255,0.04)",
                    borderLeft: `3px solid ${CANDIDATE_COLORS[((s.candidateIndex ?? 1) - 1) % CANDIDATE_COLORS.length]}`,
                    paddingLeft: 4,
                  }}
                  title={`candidate #${s.candidateIndex}`}
                >
                  {s.text}
                </span>
              ),
            )
          : a.raw}
      </pre>
      <div style={{ fontSize: 10.5, color: C_MUTED, marginTop: 3 }}>{capNote(a)}</div>
    </>
  );
};

/** Honest, specific annotations for rejected rows that are NOT bursts. */
function notConfettiNotes(a: Analyzed): string[] {
  const notes: string[] = [];
  if (a.placeholder) return notes;
  if (a.candidates.length === 1) {
    const p = a.candidates[0].parsed as Record<string, unknown> | unknown[];
    if (!Array.isArray(p) && typeof p.decision === "string" && p.decision.trim() === "") {
      notes.push(
        'The single JSON candidate is the prompt\'s own empty template echoed back (decision: ""). One candidate is not a burst — the gate rejected the empty decision.',
      );
    } else {
      notes.push("Exactly 1 JSON candidate — not a burst (prose around one object is normal per the real rule).");
    }
  } else if (a.candidates.length === 0 && a.raw.trim() !== "") {
    notes.push("0 parseable non-empty JSON candidates in the stored text — not a burst.");
  }
  if (a.source === "S10" && /invalid decision value 'decide'/i.test(a.row.reasoning_preview)) {
    notes.push(
      "The gate rejected the value 'decide' — the first candidate of the documented burst pattern (Decide → Reject → Proceed → Fail → Accept). No raw response was stored on this line, so this is NOT counted as a confirmed burst.",
    );
  }
  if (a.source === "S11" && a.raw.startsWith('{"error"')) {
    notes.push("The stored text is a provider error payload, not model output.");
  }
  return notes;
}

const PAGE_SIZE = 25;

export const ConfettiViewer: React.FC<ConfettiViewerProps> = ({ projectId }) => {
  const [loading, setLoading] = useState(true);
  const [s10, setS10] = useState<SourceState<DecisionReviewRow>>(emptySource());
  const [s11, setS11] = useState<SourceState<ZeroShotCallRow>>(emptySource());
  const [fetchedAt, setFetchedAt] = useState<string | null>(null);
  const [onlyProject, setOnlyProject] = useState(false);
  const [source, setSource] = useState<"all" | "S10" | "S11">("all");
  const [shown, setShown] = useState(PAGE_SIZE);

  const load = useCallback(async () => {
    setLoading(true);
    const [r10, r11] = await Promise.allSettled([
      fetchAllPages<DecisionReviewRow>((o) => fetchDecisionReviews(o)),
      fetchAllPages<ZeroShotCallRow>((o) =>
        fetchZeroShotCalls({ ...o, project_id: onlyProject && projectId !== null ? projectId : undefined }),
      ),
    ]);
    setS10(
      r10.status === "fulfilled"
        ? { rows: r10.value.rows, capped: r10.value.capped, error: null }
        : { rows: [], capped: false, error: r10.reason instanceof Error ? r10.reason.message : String(r10.reason) },
    );
    setS11(
      r11.status === "fulfilled"
        ? { rows: r11.value.rows, capped: r11.value.capped, error: null }
        : { rows: [], capped: false, error: r11.reason instanceof Error ? r11.reason.message : String(r11.reason) },
    );
    setFetchedAt(new Date().toLocaleTimeString());
    setLoading(false);
  }, [onlyProject, projectId]);

  useEffect(() => {
    void load();
  }, [load]);

  const analyzed = useMemo<Analyzed[]>(() => {
    const all = [...s10.rows.map(analyzeDecision), ...s11.rows.map(analyzeCall)];
    return all.sort((x, y) => (x.ts < y.ts ? 1 : x.ts > y.ts ? -1 : 0)); // newest first
  }, [s10.rows, s11.rows]);

  const visible = useMemo(() => analyzed.filter((a) => source === "all" || a.source === source), [analyzed, source]);
  const bursts = useMemo(() => visible.filter((a) => a.burst), [visible]);
  const rejected = useMemo(() => visible.filter((a) => !a.burst && isRejectedOrFailed(a)), [visible]);

  const burstS10 = analyzed.filter((a) => a.source === "S10" && a.burst).length;
  const burstS11 = analyzed.filter((a) => a.source === "S11" && a.burst).length;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4, color: C_BODY }}>
      <div style={{ display: "flex", alignItems: "center", gap: 12, flexWrap: "wrap", marginBottom: 8 }}>
        <div style={{ fontSize: 13.5, fontWeight: 700, color: C_TEXT }}>Confetti-burst viewer</div>
        <button
          onClick={() => void load()}
          disabled={loading}
          style={{ background: "transparent", color: C_BODY, border: `1px solid ${C_BORDER}`, borderRadius: 6, padding: "2px 10px", fontSize: 12, cursor: loading ? "default" : "pointer" }}
        >
          {loading ? "Loading…" : "Refresh"}
        </button>
        {(["all", "S10", "S11"] as const).map((k) => (
          <button
            key={k}
            onClick={() => {
              setSource(k);
              setShown(PAGE_SIZE);
            }}
            style={{
              background: source === k ? C_BORDER : "transparent",
              color: source === k ? C_TEXT : C_MUTED,
              border: `1px solid ${C_BORDER}`,
              borderRadius: 6,
              padding: "2px 10px",
              fontSize: 12,
              cursor: "pointer",
            }}
          >
            {k === "all" ? "Both stores" : k === "S10" ? "S10 decision reviews" : "S11 model calls"}
          </button>
        ))}
        <label
          style={{ display: "flex", alignItems: "center", gap: 5, fontSize: 12, color: projectId === null ? C_BORDER : C_MUTED, cursor: projectId === null ? "default" : "pointer" }}
          title="S11 rows carry project_id (filtered server-side). S10 decision reviews carry no project id and are always shown."
        >
          <input type="checkbox" style={{ margin: 0 }} disabled={projectId === null} checked={onlyProject} onChange={() => setOnlyProject((v) => !v)} />
          S11 only for project {projectId ?? "—"}
        </label>
        {fetchedAt && <span style={{ fontSize: 11, color: C_MUTED }}>fetched {fetchedAt}</span>}
      </div>

      <details style={{ ...card, marginBottom: 6 }}>
        <summary style={{ cursor: "pointer", fontSize: 12.5, color: C_TEXT }}>
          What counts as a confetti burst (real rule) and what this viewer can see
        </summary>
        <div style={{ ...wrap, fontSize: 12, lineHeight: 1.6, marginTop: 6 }}>
          <p style={{ margin: "0 0 6px" }}>
            A burst is one model response holding <b>more than one distinct, non-empty, parseable JSON candidate</b> — e.g.{" "}
            <code>Decide 0.9 → Reject 0.8 → Proceed 0.7 → Fail 0.6 → Accept 0.5</code>, interleaved with prose and a bare <code>{"{}"}</code>. It is a
            generation-quality failure: the gate retries, then walks the fallback chain, and never picks a candidate out of the noise.
          </p>
          <p style={{ margin: "0 0 6px" }}>
            <b>S10 rule</b> (decision_review.rs <code>is_confetti</code>): more than one non-empty JSON <i>object</i>. <b>S11 rule</b> (mod.rs{" "}
            <code>is_unusable_pipeline9_result</code>): more than one non-empty JSON <i>object or array</i>. Both scan from every opening brace, so an
            object holding a non-empty nested object counts as two candidates — candidates marked "nested" show when that is what happened.
          </p>
          <p style={{ margin: 0, color: C_MUTED }}>
            Limits: only the stored <b>preview</b> exists (first 500 characters; 300 for S11 error payloads), so a burst whose later candidates fall past the
            cap cannot be detected — this can under-count, never over-count. Executor placeholders and provider error payloads are labelled, not treated as
            model output.
          </p>
        </div>
      </details>

      {(s10.error || s11.error) && (
        <div style={{ ...card, borderColor: C_BAD, color: C_BAD, fontSize: 12 }}>
          {s10.error && <div>Could not load S10 decision reviews: {s10.error}</div>}
          {s11.error && <div>Could not load S11 model calls: {s11.error}</div>}
        </div>
      )}
      {(s10.capped || s11.capped) && (
        <div style={{ ...card, borderColor: C_WARN, color: C_WARN, fontSize: 12 }}>
          A store returned {MAX_PAGES * PAGE}+ rows; only the first {MAX_PAGES * PAGE} (oldest) were scanned, so newer rows are not included.
        </div>
      )}

      {!loading && !s10.error && !s11.error && (
        <div style={{ fontSize: 12, color: C_MUTED, marginBottom: 6 }}>
          Scanned <b style={{ color: C_BODY }}>{s10.rows.length}</b> S10 decision review{s10.rows.length === 1 ? "" : "s"} and{" "}
          <b style={{ color: C_BODY }}>{s11.rows.length}</b> S11 model call{s11.rows.length === 1 ? "" : "s"} · confetti bursts found:{" "}
          <b style={{ color: burstS10 + burstS11 > 0 ? C_WARN : C_BODY }}>{burstS10}</b> in S10, <b style={{ color: burstS10 + burstS11 > 0 ? C_WARN : C_BODY }}>{burstS11}</b> in S11
        </div>
      )}

      <div style={{ fontSize: 12.5, fontWeight: 700, color: C_TEXT, margin: "6px 0 6px" }}>Confirmed confetti bursts</div>
      {loading && <div style={{ fontSize: 12, color: C_MUTED }}>Loading capture stores…</div>}
      {!loading && bursts.length === 0 && (
        <div style={{ ...card, fontSize: 12, lineHeight: 1.6 }}>
          <div style={{ color: C_TEXT, fontWeight: 600, marginBottom: 4 }}>No confetti bursts in the stored data.</div>
          <div style={{ color: C_MUTED }}>
            None of the {visible.length} scanned row{visible.length === 1 ? "" : "s"} contain more than one non-empty JSON candidate under the real rule.
            A burst would appear here with its stored raw response, every parsed candidate (decision, confidence, reasoning) colour-segmented against the
            surrounding prose, the candidate the gate committed to (when it can be matched exactly), and the model that produced it. Look for a stored
            response containing several conflicting JSON objects in a row — the documented Decide → Reject → Proceed → Fail → Accept sequence.
          </div>
          <div style={{ color: C_MUTED, marginTop: 6 }}>
            Bursts only become visible when the model actually produces one and the executor stores the raw text; rows whose raw response was never stored
            (older lines, or a review that failed before any generation) can never be confirmed either way.
          </div>
        </div>
      )}
      {bursts.map((a, i) => (
        <div key={`b-${a.source}-${a.ts}-${i}`} style={{ ...card, borderColor: C_WARN }}>
          <CardHeader a={a} extra={<span style={chip(C_WARN)}>CONFETTI · {a.candidates.length} candidates</span>} />
          {a.candidates.filter((o) => o.nestedIn === null).length <= 1 && (
            <div style={{ fontSize: 11.5, color: C_WARN, marginBottom: 4 }}>
              Note: all but one candidate are nested inside another — the real rule flags this, but it may be a single object with a nested object rather than
              competing answers.
            </div>
          )}
          <GateSummary a={a} />
          <RawText a={a} segmented />
          <Candidates a={a} />
        </div>
      ))}

      <div style={{ fontSize: 12.5, fontWeight: 700, color: C_TEXT, margin: "14px 0 2px" }}>
        Recent rejected / failed responses <span style={{ ...chip(C_MUTED), marginLeft: 6 }}>NOT confetti</span>
      </div>
      <div style={{ fontSize: 11.5, color: C_MUTED, marginBottom: 6 }}>
        Gate outcomes with no usable judgment (S10) and unsuccessful calls (S11) that do not meet the burst rule — shown so the panel stays useful, and
        labelled with why each is not a burst.
      </div>
      {!loading && rejected.length === 0 && (
        <div style={{ fontSize: 12, color: C_MUTED }}>No rejected or failed rows in the selected stores.</div>
      )}
      {rejected.slice(0, shown).map((a, i) => {
        const notes = notConfettiNotes(a);
        return (
          <div key={`r-${a.source}-${a.ts}-${i}`} style={card}>
            <CardHeader a={a} extra={<span style={chip(C_MUTED)}>{a.candidates.length} JSON candidate{a.candidates.length === 1 ? "" : "s"}</span>} />
            <GateSummary a={a} />
            <RawText a={a} segmented={false} />
            {notes.map((n, k) => (
              <div key={k} style={{ ...wrap, fontSize: 11.5, color: C_MUTED, marginTop: 4 }}>
                • {n}
              </div>
            ))}
          </div>
        );
      })}
      {rejected.length > shown && (
        <button
          onClick={() => setShown((n) => n + PAGE_SIZE)}
          style={{ alignSelf: "flex-start", background: "transparent", color: C_BODY, border: `1px solid ${C_BORDER}`, borderRadius: 6, padding: "3px 12px", fontSize: 12, cursor: "pointer" }}
        >
          Show {Math.min(PAGE_SIZE, rejected.length - shown)} more ({rejected.length - shown} remaining)
        </button>
      )}
    </div>
  );
};

export default ConfettiViewer;
