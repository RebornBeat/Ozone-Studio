/**
 * Capture stores (S10 decision_review.jsonl, S11 zero_shot_calls.jsonl) via the
 * B4/B5 routes. Shapes confirmed against the live host. NOTE: every row on
 * disk today is a failure case (OpenRouter credits exhausted) — consumers must
 * treat failure as a primary state. There is no `total` in the response;
 * page until fewer than `limit` rows come back.
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
