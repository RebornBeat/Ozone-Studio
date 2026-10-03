/**
 * E4 — shared failure / ReviewPending classification + badge for the capture
 * viewers (E1 model calls, E2 decision reviews, E3 correlation, E6 confetti).
 *
 * Doctrine: NEVER soften a real failure into looking like a success, and a
 * success that needed retries/fallback must still say so. Every `reasons[]`
 * entry is derived from the row's own fields (or literal text found in its
 * recorded preview) — nothing is guessed.
 *
 * Real semantics this classifier encodes (verified by direct read):
 *
 * S11 zero_shot_calls.jsonl — `capture_zero_shot_call` (src/orchestrator/mod.rs
 * ~2592-2648) called from `metered_execute_resilient` (~2560-2585):
 *   - Primary call runs, then up to 2 same-model retries while
 *     `is_unusable_pipeline9_result`; if still unusable → `used_fallback = true`
 *     and `try_fallback_chain` runs. `retry_count` = same-model retries only.
 *   - `success` = `!is_unusable_pipeline9_result` on the FINAL result
 *     (mod.rs:2339): unusable = hard Err, empty/blank response text, OR a
 *     response with >1 non-empty parseable JSON candidate ("confetti").
 *     So success=true means "non-empty, single-candidate" — NOT "the content was
 *     semantically valid"; the calling stage judges that, and it isn't recorded.
 *   - `model_used` is "" for every Err result (and when the pipeline omitted it);
 *     `response_preview` is the error string cut to 300 chars for Err, or the
 *     response text cut to 500 chars for Ok.
 *   - success=false + used_fallback=true ⇒ the whole fallback chain also failed.
 *
 * S10 decision_review.jsonl — `DecisionReviewExecutor::capture`
 * (src/orchestrator/decision_review.rs ~317-345) called from `multi_pass_review`
 * and `execute`. Real `decision` values:
 *   - "proceed" / "decline": a valid judgment. Per-chunk rows carry the model's
 *     own confidence and a "[chunk i/n]" suffix on task_summary_preview; the
 *     MERGED final row (no chunk marker, raw response "") carries a fixed
 *     merge-rule confidence (0.8 proceed / 0.7 decline), not a model score.
 *   - "chunk-no-judgment": that chunk's response had no candidate with a valid
 *     decision + real reasoning (placeholder "..." rejected) — an unusable call.
 *   - "review_pending": merged result when NO chunk produced a valid judgment;
 *     the gate returns ReviewPending, never a fabricated Proceed.
 *   - "review-failed": every review model failed before/while generating;
 *     model_used "none (all models failed)"; the executor returns ReviewPending
 *     to the caller (fail-closed-to-human) and `reasoning_preview` holds the error.
 *
 * Confetti (docs/ZERO_SHOT_CALL_REGISTRY.md §13; methodology 35): a burst of
 * several conflicting JSON candidates in one response. Treated as unusable by
 * design — a generation-quality problem to re-ask, never to "pick the plausible
 * one". Verified against live rows: every current row in both stores is a
 * failure (provider "Insufficient credits" errors for S11; review-failed for S10).
 */
import React from "react";
import type { DecisionReviewRow, ToolCallRow, ZeroShotCallRow } from "../../data/captureData";

export type CaptureSeverity = "success" | "fallback" | "failure" | "review-pending" | "unclassified";
export interface CaptureStatus {
  severity: CaptureSeverity;
  label: string;
  reasons: string[];
}

// ─────────────────────────────────────────────────────────────────────────
// Presentation
// ─────────────────────────────────────────────────────────────────────────

export const SEVERITY_STYLE: Record<CaptureSeverity, { color: string; glyph: string; word: string }> = {
  failure: { color: "#ff8a8a", glyph: "✕", word: "Failure" },
  fallback: { color: "#e8c14f", glyph: "⇄", word: "Fallback/retry" },
  success: { color: "#8fe38f", glyph: "✓", word: "Success" },
  "review-pending": { color: "#8fa8d8", glyph: "⏸", word: "Review pending" },
  unclassified: { color: "var(--color-text-muted)", glyph: "?", word: "Unclassified" },
};

/** Sort key: most attention-worthy first (failure, review-pending, fallback, unclassified, success). */
const RANK: Record<CaptureSeverity, number> = { failure: 0, "review-pending": 1, fallback: 2, unclassified: 3, success: 4 };
export const severityRank = (s: CaptureSeverity): number => RANK[s];

export function summarizeSeverities(statuses: CaptureStatus[]): Record<CaptureSeverity, number> {
  const out: Record<CaptureSeverity, number> = { failure: 0, fallback: 0, success: 0, "review-pending": 0, unclassified: 0 };
  for (const s of statuses) out[s.severity] += 1;
  return out;
}

export const CaptureStatusBadge: React.FC<{ status: CaptureStatus }> = ({ status }) => {
  const st = SEVERITY_STYLE[status.severity];
  const tip = status.reasons.length ? status.reasons.join("\n") : status.label;
  return (
    <span
      title={tip}
      aria-label={`${st.word}: ${status.label}. ${status.reasons.join(" ")}`}
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: 5,
        maxWidth: "100%",
        padding: "1px 8px",
        borderRadius: 999,
        border: `1px ${status.severity === "unclassified" ? "dashed" : "solid"} ${st.color}`,
        color: st.color,
        fontSize: 11,
        lineHeight: 1.6,
        whiteSpace: "nowrap",
      }}
    >
      <span aria-hidden="true" style={{ fontWeight: 700 }}>
        {st.glyph}
      </span>
      <span style={{ overflow: "hidden", textOverflow: "ellipsis" }}>{status.label}</span>
    </span>
  );
};

// ─────────────────────────────────────────────────────────────────────────
// Text analysis helpers (exported: E6's confetti viewer needs them too)
// ─────────────────────────────────────────────────────────────────────────

/** End index of the balanced (string-aware) span starting at `start`, or -1. */
function balancedEnd(text: string, start: number, open: string, close: string): number {
  let depth = 0;
  let inString = false;
  let escaped = false;
  for (let i = start; i < text.length; i++) {
    const c = text[i];
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
      if (depth === 0) return i;
    }
  }
  return -1;
}

function nonEmptyJson(candidate: string): boolean {
  try {
    const v = JSON.parse(candidate);
    if (Array.isArray(v)) return v.length > 0;
    return v !== null && typeof v === "object" && Object.keys(v).length > 0;
  } catch {
    return false;
  }
}

/**
 * Distinct TOP-LEVEL non-empty parseable JSON objects/arrays in a response (the
 * intended meaning of "candidate": a nested object inside another candidate is
 * not a separate candidate). Empty `{}` / `[]` are skipped, as the gate does.
 */
export function countJsonCandidates(text: string): { count: number; candidates: string[] } {
  const found: { start: number; s: string }[] = [];
  let i = 0;
  while (i < text.length) {
    const c = text[i];
    if (c === "{" || c === "[") {
      const end = balancedEnd(text, i, c, c === "{" ? "}" : "]");
      if (end !== -1) {
        const cand = text.slice(i, end + 1);
        if (nonEmptyJson(cand)) {
          found.push({ start: i, s: cand });
          i = end + 1;
          continue;
        }
      }
    }
    i++;
  }
  found.sort((a, b) => a.start - b.start);
  return { count: found.length, candidates: found.map((f) => f.s) };
}

/**
 * Literal port of the host's `extract_all_json_candidates_shared`
 * (src/orchestrator/mod.rs:2370) — the count `is_unusable_pipeline9_result`
 * actually uses. It starts a scan at EVERY open brace/bracket without skipping
 * past a found end, so nested non-empty values are counted as extra candidates.
 * Exported so viewers can show WHY the host judged a response unusable.
 */
export function hostCandidateCount(text: string): number {
  let n = 0;
  for (const [open, close] of [
    ["{", "}"],
    ["[", "]"],
  ] as const) {
    for (let start = 0; start < text.length; start++) {
      if (text[start] !== open) continue;
      const end = balancedEnd(text, start, open, close);
      if (end !== -1 && nonEmptyJson(text.slice(start, end + 1))) n++;
    }
  }
  return n;
}

export interface ProviderError {
  /** The provider's own message text when one could be extracted. */
  message: string | null;
  /** True when the whole preview parsed as JSON (false when truncated/plain text). */
  parsedJson: boolean;
}

/** Best-effort extraction of a provider error message from a recorded preview. Never invents text. */
export function extractProviderError(preview: string): ProviderError {
  const t = (preview ?? "").trim();
  if (!t) return { message: null, parsedJson: false };
  const fromInner = (inner: string): string | null => {
    const brace = inner.indexOf("{");
    if (brace !== -1) {
      try {
        const e = JSON.parse(inner.slice(brace));
        const m = e?.error?.message ?? e?.message;
        if (typeof m === "string") return m;
      } catch {
        /* fall through */
      }
    }
    return inner;
  };
  try {
    const v = JSON.parse(t);
    if (v && typeof v === "object") {
      const err = (v as any).error;
      if (typeof err === "string") return { message: fromInner(err), parsedJson: true };
      if (err && typeof err === "object" && typeof err.message === "string") return { message: err.message, parsedJson: true };
      if (typeof (v as any).message === "string") return { message: (v as any).message, parsedJson: true };
    }
    return { message: null, parsedJson: true };
  } catch {
    // Truncated (previews are cut to 300/500 chars): pull the last quoted "message" value if present.
    const m = t.match(/message\\*"\s*:\s*\\*"([^\\"]*)/);
    return { message: m ? m[1] : null, parsedJson: false };
  }
}

/** Literal-evidence failure categories: only reported when the recorded text really says so. */
const EVIDENCE_PATTERNS: { re: RegExp; category: string }[] = [
  { re: /insufficient credits?/i, category: "provider reports insufficient credits" },
  { re: /rate.?limit|too many requests|\b429\b/i, category: "rate limit signalled" },
  { re: /unauthori[sz]ed|invalid api key|forbidden|\b401\b|\b403\b/i, category: "authentication/authorization error signalled" },
  { re: /timed?[ -]?out|timeout/i, category: "timeout signalled" },
  { re: /connection (?:refused|reset)|econnrefused|could not connect/i, category: "connection failure signalled" },
];

function evidenceReasons(text: string): string[] {
  const out: string[] = [];
  for (const { re, category } of EVIDENCE_PATTERNS) {
    const m = text.match(re);
    if (m) out.push(`Recorded text matches “${m[0]}” — ${category}.`);
  }
  return out;
}

const clip = (s: string, n: number): string => (s.length > n ? `${s.slice(0, n)}…` : s);
const plural = (n: number, one: string, many: string): string => `${n} ${n === 1 ? one : many}`;

// ─────────────────────────────────────────────────────────────────────────
// S11 — zero-shot call rows
// ─────────────────────────────────────────────────────────────────────────

export function classifyZeroShotCall(row: ZeroShotCallRow): CaptureStatus {
  if (typeof row.success !== "boolean") {
    return { severity: "unclassified", label: "unclassified", reasons: ["Row has no boolean `success` field, so its outcome cannot be determined."] };
  }
  const model = (row.model_used ?? "").trim();
  const retries = typeof row.retry_count === "number" ? row.retry_count : 0;
  const preview = row.response_preview ?? "";

  if (!row.success) {
    const reasons: string[] = [];
    if (!model) reasons.push("No model produced a usable answer (`model_used` is empty).");
    if (row.used_fallback) {
      reasons.push(
        `The primary model was unusable after ${plural(retries, "same-model retry", "same-model retries")}, and the fallback chain then also failed.`,
      );
    } else {
      reasons.push(`Failed with no fallback recorded (${plural(retries, "retry", "retries")}).`);
    }

    // Why the FINAL result was judged unusable — from the recorded preview only.
    if (preview.trim() === "") {
      reasons.push("Recorded response text is empty (an empty response is unusable by definition).");
    } else {
      const top = countJsonCandidates(preview);
      const host = hostCandidateCount(preview);
      const err = extractProviderError(preview);
      if (top.count > 1) {
        reasons.push(
          `Confetti: ${top.count} separate top-level JSON candidates in one response — unusable by design (re-ask, never pick one).`,
        );
      } else if (host > 1 && top.count === 1) {
        reasons.push(
          `One top-level JSON value with nested objects. The host's scanner (extract_all_json_candidates_shared) counts nested values as separate candidates (${host}), which may be why it was judged unusable.`,
        );
      }
      if (err.message) reasons.push(`Provider/error message recorded: “${clip(err.message, 200)}”.`);
      reasons.push(...evidenceReasons(err.message ?? preview));
      if (top.count <= 1 && host <= 1 && !err.message) {
        reasons.push(`Recorded response/error text: “${clip(preview, 200)}”.`);
      }
      if (preview.length >= 500) reasons.push("Preview is truncated at 500 characters, so counts above are lower bounds.");
    }
    return { severity: "failure", label: model ? `failed (${model})` : "failed — no model answered", reasons };
  }

  // success === true
  if (row.used_fallback) {
    return {
      severity: "fallback",
      label: model ? `answered by fallback: ${model}` : "answered by fallback (model not reported)",
      reasons: [
        `The primary model was unusable after ${plural(retries, "same-model retry", "same-model retries")}; the fallback chain produced this answer.`,
        ...(model ? [] : ["The pipeline did not report which model answered."]),
      ],
    };
  }
  if (retries > 0) {
    return {
      severity: "fallback",
      label: `succeeded after ${plural(retries, "retry", "retries")}`,
      reasons: [
        `The first ${plural(retries, "attempt was", "attempts were")} unusable (empty or confetti); the same model then answered.`,
      ],
    };
  }
  return {
    severity: "success",
    label: model ? `ok (${model})` : "ok",
    reasons: [
      "Non-empty, single-candidate response on the first attempt. Whether its content was semantically valid is judged by the calling stage and is not recorded here.",
      ...(model ? [] : ["The pipeline did not report which model answered."]),
    ],
  };
}

// ─────────────────────────────────────────────────────────────────────────
// S10 — decision review rows
// ─────────────────────────────────────────────────────────────────────────

const CHUNK_MARKER = /\[chunk (\d+)\/(\d+)\]\s*$/;

export function classifyDecisionReview(row: DecisionReviewRow): CaptureStatus {
  const decision = (row.decision ?? "").trim().toLowerCase().replace(/_/g, "-");
  if (!decision) {
    return { severity: "unclassified", label: "unclassified", reasons: ["Row has no `decision` value."] };
  }
  const model = (row.model_used ?? "").trim();
  const reasoning = (row.reasoning_preview ?? "").trim();
  const chunk = (row.task_summary_preview ?? "").match(CHUNK_MARKER);
  const chunkText = chunk ? `chunk ${chunk[1]}/${chunk[2]}` : null;
  const raw = row.raw_response_preview;
  const modelNote =
    model === "unknown"
      ? ["`model_used` is “unknown” — the recorder's fallback when the pipeline response carried no model."]
      : [];

  switch (decision) {
    case "review-failed":
      return {
        severity: "failure",
        label: model.startsWith("none") ? "review failed — all models failed" : `review failed (${model || "no model"})`,
        reasons: [
          "Every review model failed; the executor returned ReviewPending to the caller (fail-closed-to-human) — needs human review.",
          ...(reasoning ? [`Recorded error: “${clip(reasoning, 240)}”.`] : ["No error text was recorded."]),
          ...evidenceReasons(reasoning),
          ...modelNote,
        ],
      };
    case "review-pending":
      return {
        severity: "review-pending",
        label: "review pending — needs human review",
        reasons: [
          reasoning || "No chunk produced a valid judgment.",
          "The gate never substitutes a Proceed: the absence of a judgment is itself the finding.",
          ...modelNote,
        ],
      };
    case "chunk-no-judgment": {
      const reasons = [
        `${chunkText ? `${chunkText[0].toUpperCase()}${chunkText.slice(1)}: ` : ""}the model's response held no candidate with a valid decision and real reasoning.`,
      ];
      if (reasoning) reasons.push(`Recorded: “${clip(reasoning, 200)}”.`);
      if (typeof raw === "string" && raw.trim()) {
        const { count } = countJsonCandidates(raw);
        reasons.push(
          count > 1
            ? `The raw response held ${count} separate JSON candidates (confetti), none valid.`
            : count === 1
              ? "The raw response held one JSON candidate, but its decision/reasoning was invalid (e.g. placeholder “...”)."
              : "The raw response held no parseable JSON object.",
        );
        if (raw.length >= 500) reasons.push("Raw response preview is truncated at 500 characters, so this is a lower bound.");
      } else if (raw === null || raw === undefined) {
        reasons.push("No raw response was recorded (older capture format).");
      }
      reasons.push(...modelNote);
      return { severity: "failure", label: chunkText ? `no judgment (${chunkText})` : "no judgment", reasons };
    }
    case "proceed":
    case "decline": {
      const merged = !chunk && raw === "";
      const reasons: string[] = [];
      if (merged) {
        reasons.push("Merged final result across chunks (safety-first: any Decline ⇒ Decline).");
        const c = row.confidence;
        if (typeof c === "number" && (Math.abs(c - 0.8) < 1e-3 || Math.abs(c - 0.7) < 1e-3)) {
          reasons.push(`Confidence ${c.toFixed(2)} is the merge rule's fixed value (0.8 proceed / 0.7 decline), not a model-reported score.`);
        }
      } else if (chunkText) {
        reasons.push(`Model judgment for ${chunkText}${typeof row.confidence === "number" ? ` (model-reported confidence ${row.confidence.toFixed(2)})` : ""}.`);
      }
      if (model.includes("+")) reasons.push(`Models involved: ${model}.`);
      reasons.push(...modelNote);
      return {
        severity: "success",
        label: `${decision}${chunkText ? ` (${chunkText})` : merged ? " (merged)" : ""}`,
        reasons: reasons.length ? reasons : ["A valid judgment was produced."],
      };
    }
    default:
      return {
        severity: "unclassified",
        label: `unclassified (${clip(row.decision, 24)})`,
        reasons: [`Unrecognized decision value “${row.decision}” — not one of proceed / decline / review_pending / chunk-no-judgment / review-failed.`],
      };
  }
}

// ─────────────────────────────────────────────────────────────────────────
// S13 — tool call rows
// ─────────────────────────────────────────────────────────────────────────

export function classifyToolCall(row: ToolCallRow): CaptureStatus {
  if (typeof row.success !== "boolean") {
    return { severity: "unclassified", label: "unclassified", reasons: ["Row has no boolean `success` field."] };
  }
  const reasons: string[] = [];
  if (!row.identity_validated) {
    reasons.push("Caller identity was not validated for this call (no session_token provided, or none was checked).");
  }
  if (!row.success) {
    const err = (row.error ?? "").trim();
    reasons.push(err ? `Recorded error: "${clip(err, 200)}".` : "Failed with no error text recorded.");
    reasons.push(...evidenceReasons(err));
    return { severity: "failure", label: `${row.tool} failed`, reasons };
  }
  reasons.push(row.identity_validated ? "Caller identity validated against AuthSystem." : "Succeeded, but caller identity was not validated.");
  return { severity: row.identity_validated ? "success" : "fallback", label: `${row.tool} ok`, reasons };
}
