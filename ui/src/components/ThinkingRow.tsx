/**
 * ThinkingRow.tsx — the in-transcript live "the run is alive" row, extracted
 * from MetaPortion.tsx (2026-11-01) so MetaPortion and ProjectChatPanel render
 * the identical indicator. Every value is real: elapsedSeconds ticks from
 * submit time, stepInfo comes from the /task/get poll once a real task record
 * exists. Before that record exists it says exactly that instead of inventing
 * a stage. When the host reports the task as `awaiting` (a consciousness/meta
 * gate could not judge yet and the task is waiting, resumable) the row says
 * so — it is never rendered as an error.
 *
 * LIVENESS (2026-11-06): while running, the row also shows how long ago ANY model call last made progress and the remaining daily free
 * quota, from the host's 3-second `liveness` frame. A long stage (Stage 2 can run many minutes) now visibly proves it is working; if no
 * model has made progress for over 2 minutes the row says so plainly ("no model activity for N s") instead of implying all is well.
 */

import React, { useEffect, useState } from 'react';
import { getGraphEventClient, type LivenessFrame } from '../graphEventClient';

export interface ThinkingStepInfo {
  completed: number;
  total: number;
  lastAction?: string;
}

export type ThinkingAwaiting = 'meta' | 'human' | null;

export interface ThinkingRowProps {
  isRunning: boolean;
  elapsedSeconds: number;
  stepInfo: ThinkingStepInfo | null;
  /** 'meta' = waiting on a consciousness/meta judgment (retrying on its own);
   * 'human' = waiting on the operator's jurisdiction confirmation. */
  awaiting?: ThinkingAwaiting;
}

export function formatElapsed(totalSeconds: number): string {
  const s = Math.max(0, Math.floor(totalSeconds));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
}

export function ThinkingRow({ isRunning, elapsedSeconds, stepInfo, awaiting = null }: ThinkingRowProps) {
  const [live, setLive] = useState<{ ago: number | null; quota: number | null; call: LivenessFrame['last_model_call'] | null; stage: LivenessFrame['last_stage'] | null; at: number } | null>(null);
  useEffect(() => {
    if (!isRunning) {
      setLive(null);
      return;
    }
    return getGraphEventClient().onLiveness((f) =>
      setLive({ ago: f.last_model_activity_ms_ago, quota: f.free_quota_remaining, call: f.last_model_call ?? null, stage: f.last_stage ?? null, at: Date.now() }));
  }, [isRunning]);
  if (!isRunning) return null;
  // the frame's age + the time since it arrived (the parent re-renders every second, so this ticks)
  const modelAgoS = live && live.ago !== null ? Math.round((live.ago + (Date.now() - live.at)) / 1000) : null;
  const label =
    awaiting === 'meta'
      ? 'Waiting on a consciousness/meta judgment, retrying…'
      : awaiting === 'human'
        ? 'Waiting on your confirmation…'
        : 'Thinking…';
  return (
    <div
      className="orchestration-thinking-row"
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        margin: '6px 0',
        fontSize: 12,
        opacity: 0.75,
      }}
    >
      <span className="orchestration-thinking-dot" />
      <span>
        {label} {formatElapsed(elapsedSeconds)}
        {stepInfo
          ? ` · step ${stepInfo.completed}/${stepInfo.total}${stepInfo.lastAction ? ` — ${stepInfo.lastAction}` : ''}`
          : ' · working — no task record yet (early stages)'}
        {modelAgoS !== null && (
          <span style={{ color: modelAgoS > 120 ? '#d9b45f' : undefined }}>
            {modelAgoS > 120 ? ` · no model activity for ${modelAgoS}s` : ` · last model activity ${modelAgoS}s ago`}
          </span>
        )}
        {live && live.quota !== null ? ` · ${live.quota} free calls left today` : ''}
        {live?.stage ? ` · stage ${live.stage.stage} ${live.stage.name}` : ''}
        {live?.call ? ` · last call ${live.call.call_site} → ${live.call.model} (${Math.round(live.call.latency_ms / 100) / 10}s, ${live.call.success ? 'ok' : 'failed'})` : ''}
      </span>
    </div>
  );
}

export default ThinkingRow;
