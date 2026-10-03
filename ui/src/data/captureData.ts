/**
 * Capture stores (S10 decision_review.jsonl, S11 zero_shot_calls.jsonl, S13
 * tool_calls.jsonl) via the B4/B5/(S13) routes. Shapes confirmed against the
 * live host. NOTE: every row on disk today is a failure case (OpenRouter
 * credits exhausted) — consumers must treat failure as a primary state.
 * There is no `total` in the response; page until fewer than `limit` rows
 * come back.
 */
import { getJson, qs } from "./http";

export interface DecisionReviewRow {
  ts: string;
  model_used: string;
  tokens_used: number;
  decision: string;
  confidence: number | null;
  task_summary_preview: string;
  reasoning_preview: string;
  /** Optional: older lines predate it. */
  raw_response_preview: string | null;
}

export interface ZeroShotCallRow {
  ts: string;
  call_site: string;
  model_used: string;
  tokens_used: number;
  retry_count: number;
  used_fallback: boolean;
  success: boolean;
  response_preview: string;
  amt_container_id: number | null;
  blueprint_id: number | null;
  project_id: number | null;
  prompt_preview: string;
}

export interface CapturePage<T> {
  limit: number;
  offset: number;
  rows: T[];
}

export function fetchDecisionReviews(opts: { offset?: number; limit?: number } = {}): Promise<CapturePage<DecisionReviewRow>> {
  return getJson(`/capture/decision-reviews${qs({ offset: opts.offset, limit: opts.limit })}`);
}

export interface ZeroShotFilters {
  offset?: number;
  limit?: number;
  amt_container_id?: number;
  blueprint_id?: number;
  project_id?: number;
  call_site?: string;
  model_used?: string;
}

export function fetchZeroShotCalls(opts: ZeroShotFilters = {}): Promise<CapturePage<ZeroShotCallRow>> {
  return getJson(`/capture/zero-shot-calls${qs({ ...opts })}`);
}

/**
 * S13 tool_calls.jsonl — every real /mcp/call, one row each. Shape confirmed
 * directly against the real writer (src/grpc/mod.rs's mcp_call handler),
 * not guessed: `identity_validated` reflects whether the caller's
 * session_token was actually verified against AuthSystem for that call (a
 * missing/absent token is honestly false, not omitted). Route landing is
 * tracked in docs/guides/coordination-ripple-gaps.md — this fetch fn is
 * built ahead of it so the wire-up is a one-line change once it exists.
 */
export interface ToolCallRow {
  ts: string;
  tool: string;
  agent: string;
  success: boolean;
  error: string;
  input_preview: string;
  identity_validated: boolean;
  transport: string;
}

export interface ToolCallFilters {
  offset?: number;
  limit?: number;
  tool?: string;
  agent?: string;
}

/**
 * Defensive on purpose, unlike the S10/S11 fetchers above: this route does
 * not exist on the host yet (tracked in
 * docs/guides/coordination-ripple-gaps.md), and the Electron IPC bridge's
 * `http.get` (electron/main.js's `rawRequest`) does not check HTTP status
 * codes at all — a 404's plain-text body fails its own JSON.parse and
 * resolves with the raw string instead of throwing. Without this check,
 * ToolCallPanel would get a string where it expects `{rows: [...]}` and
 * crash confusingly instead of showing the honest "route not live yet"
 * state it's built to show. Remove this extra check if/when the bridge
 * itself is ever made status-aware — it would become redundant, not wrong.
 */
export async function fetchToolCalls(opts: ToolCallFilters = {}): Promise<CapturePage<ToolCallRow>> {
  const result = await getJson<unknown>(`/capture/tool-calls${qs({ ...opts })}`);
  if (!result || typeof result !== "object" || !Array.isArray((result as CapturePage<ToolCallRow>).rows)) {
    throw new Error(
      typeof result === "string"
        ? `unexpected response (route likely not live yet): ${result.slice(0, 120)}`
        : "unexpected response shape from /capture/tool-calls",
    );
  }
  return result as CapturePage<ToolCallRow>;
}
