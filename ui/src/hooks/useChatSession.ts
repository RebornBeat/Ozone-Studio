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

import { useCallback, useRef, useState } from 'react';
import { TranscriptEntry } from '../chatRendering';

export interface UseChatSessionArgs {
  projectId: number;
  workspaceId?: number;
}

export function useChatSession({ projectId, workspaceId }: UseChatSessionArgs) {
  const [transcript, setTranscript] = useState<TranscriptEntry[]>([]);
  const [isRunning, setIsRunning] = useState(false);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const stopPolling = useCallback(() => {
    if (pollRef.current) {
      clearInterval(pollRef.current);
      pollRef.current = null;
    }
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

      // Same lightweight task-discovery poll MetaPortion uses: /orchestrate
      // blocks until the whole run finishes, so this just lets a future
      // step-progress UI (not built here — this panel has no step display
      // today) latch onto the real task id if one ever gets added.
      let foundTaskId: number | null = null;
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
          ? {
              ...entry,
              content: result.response || result.error || 'No response generated',
              modelUsed: result.model_used,
              stageCount: Array.isArray(result.stages_completed)
                ? result.stages_completed.length
                : undefined,
              thinkingLog: Array.isArray(result.thinking_log)
                ? result.thinking_log
                : undefined,
              amtSummary: result.amt_summary ?? undefined,
              attemptTrail: Array.isArray(result.attempt_trail) ? result.attempt_trail : undefined,
              refusals: Array.isArray(result.refusals) ? result.refusals : undefined,
            }
          : entry
      ));
    } catch (e: any) {
      stopPolling();
      setTranscript(prev => prev.map(entry =>
        entry.id === placeholderId
          ? { ...entry, content: e?.message || 'Request failed' }
          : entry
      ));
    } finally {
      setIsRunning(false);
    }
  }, [projectId, workspaceId, stopPolling]);

  return { transcript, isRunning, send };
}
