/**
 * useChatSession.ts — independent chat-session hook for ProjectChatPanel.tsx.
 *
 * Calls the same real window.ozone.orchestrate + task-list-polling path
 * MetaPortion.tsx's own handleSubmit uses, but as a genuinely separate
 * implementation — not a refactor of that function, which stays in
 * MetaPortion since it's entangled with voice capture and consciousness/
 * emotion triggers specific to the global sidebar and don't apply here.
 * This hook keeps its own transcript, scoped to one project/workspace,
 * independent of MetaPortion's own per-context transcripts.
 */

import { useCallback, useEffect, useRef, useState } from 'react';
import { TranscriptEntry } from '../chatRendering';

export interface UseChatSessionArgs {
  projectId: number;
  workspaceId?: number;
}

/** Normalized shape both the fast synchronous /orchestrate path and the slow
 * /task/get recovery path reduce to before touching the transcript. One
 * function applies either to an entry so the two paths can never render
 * this information inconsistently. */
interface NormalizedOrchestrationResult {
  response?: string | null;
  error?: string | null;
  modelUsed?: string | null;
  stageCount?: number;
  thinkingLog?: any[];
  amtSummary?: any;
  attemptTrail?: string[];
  refusals?: string[];
}

function applyResultToEntry(
  entry: TranscriptEntry,
  result: NormalizedOrchestrationResult,
): TranscriptEntry {
  return {
    ...entry,
    content: result.response || result.error || 'No response generated',
    modelUsed: result.modelUsed ?? undefined,
    stageCount: result.stageCount,
    thinkingLog: Array.isArray(result.thinkingLog) ? result.thinkingLog : undefined,
    amtSummary: result.amtSummary ?? undefined,
    attemptTrail: Array.isArray(result.attemptTrail) ? result.attemptTrail : undefined,
    refusals: Array.isArray(result.refusals) ? result.refusals : undefined,
  };
}

// Real orchestration can walk a multi-model fallback chain across a
// multi-stage pipeline well past the Electron client's own generous
// SLOW_ENDPOINT_TIMEOUT_MS (ui/electron/main.js, 5 min for /orchestrate) —
// confirmed live via host.log: task 655 completed successfully 585357ms
// (~9m45s) after it started, well past that timeout, while the user only
// ever saw a bare "Error: timeout" in the UI. The result was real and fully
// computed the whole time; only the client gave up waiting for it. When the
// synchronous call times out but the task-discovery poll below already
// found the real task_id, this recovery poll picks the real result back up
// from /task/get instead of discarding it.
const RECOVERY_POLL_INTERVAL_MS = 5000; // a few seconds — catches completion promptly without hammering /task/get (a lightweight call, default 10s IPC timeout, not in SLOW_ENDPOINTS)
// 20 minutes: more than 2x the real 585357ms (~9m45s) case above, with
// margin because the actual slowness driving this is external AI-provider
// degradation (OpenRouter free-tier 429s / empty responses, visible
// throughout host.log) that can get worse, not better. This is a generous,
// bounded ceiling, not another short timeout that would reintroduce the
// same bug — if it's ever hit, the message says so honestly.
const RECOVERY_POLL_CEILING_MS = 20 * 60 * 1000;

// "interrupted" (src/task/mod.rs boot reconciliation) means the host process
// ended mid-task — it will never resume or complete on its own, so it's
// terminal for polling purposes same as completed/failed/cancelled, even
// though it isn't a status the task reached via its own normal lifecycle.
function isTerminalTaskStatus(status: unknown): boolean {
  const s = String(status ?? '').toLowerCase();
  return (
    s.includes('completed') ||
    s.includes('failed') ||
    s.includes('cancelled') ||
    s.includes('canceled') ||
    s.includes('interrupted')
  );
}

export function useChatSession({ projectId, workspaceId }: UseChatSessionArgs) {
  // PERSISTENT CHAT (2026-10-27 — the operator switched tabs and the
  // conversation was gone): the transcript survives tab switches via
  // sessionStorage, keyed by project+workspace so different projects never
  // bleed into each other. Cleared explicitly on project change.
  const chatKey = `ozone-chat:${projectId ?? 'none'}:${workspaceId ?? 'none'}`;
  const [transcript, setTranscript] = useState<TranscriptEntry[]>(() => {
    try {
      const saved = sessionStorage.getItem(chatKey);
      return saved ? (JSON.parse(saved) as TranscriptEntry[]) : [];
    } catch { return []; }
  });
  useEffect(() => {
    try { sessionStorage.setItem(chatKey, JSON.stringify(transcript)); } catch { /* quota — chat still works, just not persisted */ }
  }, [transcript, chatKey]);
  const [isRunning, setIsRunning] = useState(false);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const stopPolling = useCallback(() => {
    if (pollRef.current) {
      clearInterval(pollRef.current);
      pollRef.current = null;
    }
  }, []);

  // Recovery path: the task was found (foundTaskId) but the synchronous
  // /orchestrate call timed out client-side before the orchestration itself
  // finished. Poll the real task record until it reaches a terminal status,
  // then render it through the SAME applyResultToEntry the fast path uses
  // below — never a second, divergent rendering of the same information.
  const recoverTaskResult = useCallback(async (taskId: number, placeholderId: number) => {
    const deadline = Date.now() + RECOVERY_POLL_CEILING_MS;
    while (Date.now() < deadline) {
      await new Promise(resolve => setTimeout(resolve, RECOVERY_POLL_INTERVAL_MS));
      try {
        const task = await (window as any).ozone?.task?.status?.(taskId);
        if (task && isTerminalTaskStatus(task.status)) {
          const status = String(task.status ?? '').toLowerCase();
          // "interrupted" tasks carry no task.error (the boot reconciliation
          // that marks them only changes status, per src/task/mod.rs) — make
          // the honest reason explicit instead of falling through to the
          // generic "No response generated".
          const error =
            task.error ??
            (status.includes('interrupted')
              ? `Task ${taskId} was interrupted (the backend process restarted or crashed before finishing) and will not complete on its own.`
              : undefined);
          setTranscript(prev => prev.map(entry =>
            entry.id === placeholderId
              ? applyResultToEntry(entry, {
                  response: task.response,
                  error,
                  modelUsed: task.model_used,
                  thinkingLog: task.thinking_log,
                  amtSummary: task.amt_summary,
                  attemptTrail: task.attempt_trail,
                  refusals: task.refusals,
                })
              : entry
          ));
          return;
        }
      } catch {
        /* transient — /task/get hiccup, keep polling until the ceiling */
      }
    }
    // Ceiling reached with no terminal status — honest, not fabricated: we
    // genuinely don't know the outcome yet.
    setTranscript(prev => prev.map(entry =>
      entry.id === placeholderId
        ? {
            ...entry,
            content: `Still running after ${Math.round(RECOVERY_POLL_CEILING_MS / 60000)} minutes (task ${taskId}) — no result yet. It may still finish; check the Tasks view, or try again.`,
          }
        : entry
    ));
  }, []);

  const send = useCallback(async (prompt: string) => {
    if (!prompt.trim() || !(window as any).ozone?.orchestrate) return;

    const userEntry: TranscriptEntry = {
      id: Date.now(),
      role: 'user',
      content: prompt,
      timestamp: Date.now(),
    };
    setTranscript(prev => [...prev, userEntry]);

    const placeholderId = Date.now() + 1;
    setTranscript(prev => [...prev, {
      id: placeholderId,
      role: 'assistant',
      content: '⏳ Thinking...',
      timestamp: Date.now(),
    }]);

    setIsRunning(true);
    // Hoisted above the try (rather than declared inside it, as before) so
    // the catch block below — the timeout-recovery path — can see whatever
    // the poll found before the orchestrate promise rejected. A `let` inside
    // the try is block-scoped to it and would not be visible in the catch.
    let foundTaskId: number | null = null;
    try {
      const orchestratePromise = (window as any).ozone.orchestrate({
        prompt,
        project_id: projectId,
        workspace_id: workspaceId,
        user_id: 1,
        device_id: 1,
        consciousness_enabled: false,
        token_budget: 100000,
      });

      // Lightweight task-discovery poll: /orchestrate blocks until the whole
      // run finishes, so this just finds the real task_id while it's still
      // running — used by the timeout-recovery path below if the orchestrate
      // call ends up timing out client-side.
      pollRef.current = setInterval(async () => {
        try {
          if (foundTaskId) return;
          const list = await (window as any).ozone?.task?.list?.();
          const running = (list?.tasks ?? [])
            .filter((t: any) => String(t.status ?? '').toLowerCase().includes('running')
              || String(t.status ?? '').toLowerCase().includes('inprogress'))
            .sort((a: any, b: any) => (b.created_at ?? 0) - (a.created_at ?? 0));
          if (running.length > 0) {
            foundTaskId = running[0].task_id;
          }
        } catch {
          /* transient — keep polling */
        }
      }, 900);

      const result = await orchestratePromise;
      stopPolling();

      setTranscript(prev => prev.map(entry =>
        entry.id === placeholderId
          ? applyResultToEntry(entry, {
              response: result.response,
              error: result.error,
              modelUsed: result.model_used,
              stageCount: Array.isArray(result.stages_completed)
                ? result.stages_completed.length
                : undefined,
              thinkingLog: result.thinking_log,
              amtSummary: result.amt_summary,
              attemptTrail: result.attempt_trail,
              refusals: result.refusals,
            })
          : entry
      ));
    } catch (e: any) {
      stopPolling();
      const isTimeout = typeof e?.message === 'string' && e.message.toLowerCase().includes('timeout');
      if (isTimeout && foundTaskId) {
        // The real orchestration may well still be running server-side (or
        // may have already finished) — don't throw away a real result the
        // client simply gave up waiting for. Switch to recovery polling
        // instead of showing a bare error.
        setTranscript(prev => prev.map(entry =>
          entry.id === placeholderId
            ? { ...entry, content: `⏳ Still working — recovering the real result (task ${foundTaskId})...` }
            : entry
        ));
        await recoverTaskResult(foundTaskId, placeholderId);
      } else {
        // foundTaskId was never discovered (the task finished or was never
        // visible during the 900ms poll window) — keep the existing
        // behavior rather than inventing a task_id to recover against.
        setTranscript(prev => prev.map(entry =>
          entry.id === placeholderId
            ? { ...entry, content: e?.message || 'Request failed' }
            : entry
        ));
      }
    } finally {
      setIsRunning(false);
    }
  }, [projectId, workspaceId, stopPolling, recoverTaskResult]);

  return { transcript, isRunning, send };
}
