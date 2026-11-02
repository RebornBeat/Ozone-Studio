/**
 * ThinkingRow.tsx — the in-transcript live "the run is alive" row, extracted
 * from MetaPortion.tsx (2026-11-01) so MetaPortion and ProjectChatPanel render
 * the identical indicator. Every value is real: elapsedSeconds ticks from
 * submit time, stepInfo comes from the /task/get poll once a real task record
 * exists. Before that record exists it says exactly that instead of inventing
 * a stage. When the host reports the task as `awaiting` (a consciousness/meta
 * gate could not judge yet and the task is waiting, resumable) the row says
 * so — it is never rendered as an error.
 */

import React from 'react';

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
  if (!isRunning) return null;
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
      </span>
    </div>
  );
}

export default ThinkingRow;
