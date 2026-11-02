/**
 * PausedBanner.tsx — shown by every chat surface (meta chat, project chat) when a task was
 * PAUSED by a failed step. The host never continues past a failure: the task waits, with
 * the error, until the user resumes it — from exactly the failed step, with completed
 * steps restored (POST /task/resume_run). The same task is also resumable from the Tasks
 * tab / project tabs (TaskDetailPanel, OrderPanel).
 */

import React from 'react';

export interface PausedInfo {
  taskId: number;
  step: number;
  error: string;
}

export function PausedBanner({
  paused,
  onResume,
  busy,
}: {
  paused: PausedInfo | null;
  onResume: () => void;
  busy?: boolean;
}) {
  if (!paused) return null;
  return (
    <div
      role="alert"
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 10,
        fontSize: 12,
        color: '#f6c177',
        background: '#241d10',
        border: '1px solid #6b5420',
        borderRadius: 8,
        padding: '6px 10px',
        margin: '4px 0',
      }}
    >
      <span style={{ flex: 1 }}>
        ⏸ Task {paused.taskId} paused at step {paused.step} — not continued past the failure.
        <span style={{ opacity: 0.85 }}> {paused.error}</span>
      </span>
      <button className="obtn" disabled={busy} onClick={onResume} title="Retry from exactly the failed step; completed steps are kept">
        Resume
      </button>
    </div>
  );
}

export default PausedBanner;
