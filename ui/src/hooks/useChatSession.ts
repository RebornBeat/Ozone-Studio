/**
 * useChatSession.ts — the ONE chat engine shared by MetaPortion (global /
 * workspace chat) and ProjectChatPanel (project-scoped chat). They are the same
 * chat with different scope; only `contextKey` / `projectId` / `workspaceId`
 * differ (2026-11-02 doctrine).
 *
 * - Per-context transcripts persisted in localStorage, restored on mount and on
 *   context switch (key = `ozone-chat:<contextKey>`; default contextKey keeps
 *   the old `${projectId}:${workspaceId}` form so existing histories restore).
 * - Task discovery by baseline id-set taken BEFORE submit, so only tasks this
 *   submit created are ever adopted. Discovery keeps retrying (up to 60 s)
 *   even after the orchestrate promise rejected with a client timeout.
 * - Shared `recoverTaskResult`: polls /task/get until terminal. Statuses
 *   awaiting / awaiting_meta / awaiting_human are NOT terminal: the task is
 *   durably waiting on a consciousness/meta judgment (or the operator's
 *   jurisdiction confirmation) and resumes on its own — keep polling, never
 *   render "Error".
 * - Server-detached walk: a client timeout never means failure.
 */

import { useCallback, useEffect, useRef, useState } from 'react';
import { TranscriptEntry } from '../chatRendering';
import { hostGet, resumeRun, parsePause } from '../ozoneClient';
import type { ThinkingAwaiting, ThinkingStepInfo } from '../components/ThinkingRow';

export interface UseChatSessionArgs {
  projectId?: number;
  workspaceId?: number;
  /** Persistence/scope key. Default: `${projectId}:${workspaceId}`. */
  contextKey?: string;
  /** Sent as model_config (e.g. { model_identifier }) when set. */
  modelConfig?: Record<string, unknown>;
  consciousnessEnabled?: boolean;
  /** Called with the raw /orchestrate (or recovered) result after the entry
   * is updated — MetaPortion uses it for speech / emotion extras. */
  onResult?: (result: any) => void | Promise<void>;
}

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
    content: result.response || result.error || '⚠ The host returned neither a response nor an error — this is a failure to answer, not an empty answer.',
    modelUsed: result.modelUsed ?? undefined,
    stageCount: result.stageCount,
    thinkingLog: Array.isArray(result.thinkingLog) ? result.thinkingLog : undefined,
    amtSummary: result.amtSummary ?? undefined,
    attemptTrail: Array.isArray(result.attemptTrail) ? result.attemptTrail : undefined,
    refusals: Array.isArray(result.refusals) ? result.refusals : undefined,
  };
}

const RECOVERY_POLL_INTERVAL_MS = 5000;
// Awaiting items can legitimately wait a long time for a meta judgment, so the
// ceiling is generous; hitting it says so honestly and the task keeps going.
const RECOVERY_POLL_CEILING_MS = 60 * 60 * 1000;
const DISCOVERY_RETRY_MS = 60 * 1000;
const DISCOVERY_INTERVAL_MS = 900;

export function isTerminalTaskStatus(status: unknown): boolean {
  const s = String(status ?? '').toLowerCase();
  if (isAwaitingTaskStatus(s)) return false;
  return (
    s.includes('completed') ||
    s.includes('failed') ||
    s.includes('cancelled') ||
    s.includes('canceled') ||
    s.includes('interrupted')
  );
}

export function isAwaitingTaskStatus(status: unknown): boolean {
  return String(status ?? '').toLowerCase().includes('awaiting');
}

function awaitingKindOf(status: unknown): ThinkingAwaiting {
  const s = String(status ?? '').toLowerCase();
  if (!s.includes('awaiting')) return null;
  return s.includes('human') ? 'human' : 'meta';
}

function isLiveTaskStatus(status: unknown): boolean {
  const s = String(status ?? '').toLowerCase();
  return s.includes('running') || s.includes('inprogress') || s.includes('pending') || isAwaitingTaskStatus(s);
}

/** Real ids from window.ozone.auth when exposed; else the legacy defaults the
 * rest of the UI filters on (OrchestrationStatusPanel/DiscoveryEffects use 1/1). */
function authIds(): { user_id: number; device_id: number } {
  const a = (window as any).ozone?.auth;
  const u = a?.getCurrentUserId?.();
  const d = a?.getCurrentDeviceId?.();
  return {
    user_id: typeof u === 'number' && u > 0 ? u : 1,
    device_id: typeof d === 'number' && d > 0 ? d : 1,
  };
}

function storageKey(contextKey: string): string {
  return `ozone-chat:${contextKey}`;
}

function loadTranscript(contextKey: string): TranscriptEntry[] {
  try {
    const saved = localStorage.getItem(storageKey(contextKey));
    return saved ? (JSON.parse(saved) as TranscriptEntry[]) : [];
  } catch {
    return [];
  }
}

export function useChatSession({
  projectId,
  workspaceId,
  contextKey: contextKeyArg,
  modelConfig,
  consciousnessEnabled = false,
  onResult,
}: UseChatSessionArgs) {
  const contextKey = contextKeyArg ?? `${projectId ?? 'none'}:${workspaceId ?? 'none'}`;

  const [byContext, setByContext] = useState<Record<string, TranscriptEntry[]>>(() => ({
    [contextKey]: loadTranscript(contextKey),
  }));
  // Restore on context switch (lazy per key).
  useEffect(() => {
    setByContext(prev => (contextKey in prev ? prev : { ...prev, [contextKey]: loadTranscript(contextKey) }));
  }, [contextKey]);
  // HOST HYDRATION (2026-11-02): the host now persists every turn (full prompt and
  // full response) as a scoped ChatTurn. When this browser has no local copy for the
  // context — new device, cleared storage, restart on another machine — rebuild the
  // transcript from the host instead of showing an empty chat.
  useEffect(() => {
    let cancelled = false;
    if (loadTranscript(contextKey).length > 0) return;
    const qs = new URLSearchParams();
    if (projectId !== undefined && projectId !== null) qs.set('project_id', String(projectId));
    if (workspaceId !== undefined && workspaceId !== null) qs.set('workspace_id', String(workspaceId));
    (async () => {
      try {
        const res: any = await hostGet(`/chat/turns?${qs.toString()}`);
        const turns: any[] = Array.isArray(res?.turns) ? res.turns : [];
        if (cancelled || turns.length === 0) return;
        const entries: TranscriptEntry[] = [];
        for (const t of turns) {
          const ts = Date.parse(t.ts) || Date.now();
          entries.push({ id: ts * 2, role: 'user', content: String(t.prompt ?? ''), timestamp: ts });
          entries.push({
            id: ts * 2 + 1,
            role: 'assistant',
            content: String(t.response ?? t.error ?? ''),
            timestamp: ts,
            modelUsed: t.model_used ?? undefined,
          });
        }
        setByContext(prev => (prev[contextKey] && prev[contextKey].length > 0 ? prev : { ...prev, [contextKey]: entries }));
      } catch { /* host unreachable or route absent: local-only chat, as before */ }
    })();
    return () => { cancelled = true; };
  }, [contextKey, projectId, workspaceId]);
  // Persist the active context whenever it changes (restart-safe).
  const transcript = byContext[contextKey] ?? [];
  useEffect(() => {
    if (!(contextKey in byContext)) return;
    try {
      localStorage.setItem(storageKey(contextKey), JSON.stringify(byContext[contextKey]));
    } catch { /* quota — chat works, just not persisted */ }
  }, [byContext, contextKey]);

  const setTranscriptFor = useCallback(
    (key: string, updater: (prev: TranscriptEntry[]) => TranscriptEntry[]) => {
      setByContext(prev => ({ ...prev, [key]: updater(prev[key] ?? loadTranscript(key)) }));
    },
    [],
  );
  const setTranscript = useCallback(
    (updater: (prev: TranscriptEntry[]) => TranscriptEntry[]) => setTranscriptFor(contextKey, updater),
    [contextKey, setTranscriptFor],
  );

  const [isRunning, setIsRunning] = useState(false);
  const [elapsedSeconds, setElapsedSeconds] = useState(0);
  const [stepInfo, setStepInfo] = useState<ThinkingStepInfo | null>(null);
  const [currentTaskId, setCurrentTaskId] = useState<number | null>(null);
  const [awaiting, setAwaiting] = useState<ThinkingAwaiting>(null);
  // FAILURE DOCTRINE: a failed step PAUSES the task (never continues). The chat shows it and
  // offers Resume, which re-enters the host walk at exactly the failed step.
  const [paused, setPaused] = useState<{ taskId: number; step: number; error: string } | null>(null);
  const tickRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const onResultRef = useRef(onResult);
  onResultRef.current = onResult;

  const stopTick = useCallback(() => {
    if (tickRef.current) {
      clearInterval(tickRef.current);
      tickRef.current = null;
    }
  }, []);
  useEffect(() => stopTick, [stopTick]);

  const applyTask = useCallback((key: string, placeholderId: number, task: any, taskId: number) => {
    const status = String(task.status ?? '').toLowerCase();
    const error =
      task.error ??
      (status.includes('interrupted')
        ? `Task ${taskId} was interrupted (the backend process restarted or crashed before finishing) and will not complete on its own.`
        : undefined);
    setTranscriptFor(key, prev => prev.map(entry =>
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
  }, [setTranscriptFor]);

  /** Poll the real task record until terminal. Awaiting statuses keep polling. */
  const recoverTaskResult = useCallback(async (taskId: number, placeholderId: number, key: string) => {
    const deadline = Date.now() + RECOVERY_POLL_CEILING_MS;
    while (Date.now() < deadline) {
      try {
        const task = await (window as any).ozone?.task?.status?.(taskId);
        if (task) {
          const steps = task.steps ?? [];
          if (steps.length > 0) {
            setStepInfo({
              completed: steps.filter((s: any) => s.status === 'completed').length,
              total: steps.length,
              lastAction: steps[steps.length - 1]?.action,
            });
          }
          const failurePause = String(task.status ?? '').toLowerCase() === 'paused' ? parsePause(task.error, taskId) : null;
          if (failurePause) {
            setAwaiting(null);
            setPaused({ taskId, step: failurePause.step, error: failurePause.error });
            setTranscriptFor(key, prev => prev.map(e => (e.id === placeholderId
              ? { ...e, content: `⏸ Task ${taskId} paused at step ${failurePause.step} — it was not continued past the failure.\n${failurePause.error}\nUse Resume to retry from exactly this step.` }
              : e)));
            return task;
          }
          if (isTerminalTaskStatus(task.status)) {
            setAwaiting(null);
            applyTask(key, placeholderId, task, taskId);
            return task;
          }
          const kind = awaitingKindOf(task.status);
          setAwaiting(kind);
          const label =
            kind === 'human'
              ? `⏳ Waiting on your confirmation (task ${taskId})…`
              : kind === 'meta'
                ? `⏳ Waiting on a consciousness/meta judgment, retrying… (task ${taskId})`
                : `⏳ Still working — recovering the real result (task ${taskId})...`;
          setTranscriptFor(key, prev => prev.map(e => (e.id === placeholderId ? { ...e, content: label } : e)));
        }
      } catch { /* transient — keep polling */ }
      await new Promise(resolve => setTimeout(resolve, RECOVERY_POLL_INTERVAL_MS));
    }
    setTranscriptFor(key, prev => prev.map(entry =>
      entry.id === placeholderId
        ? {
            ...entry,
            content: `Still running after ${Math.round(RECOVERY_POLL_CEILING_MS / 60000)} minutes (task ${taskId}) — no result yet. It may still finish; check the Tasks view.`,
          }
        : entry
    ));
    return null;
  }, [applyTask, setTranscriptFor]);

  const stop = useCallback(async () => {
    if (!currentTaskId) return;
    try { await (window as any).ozone?.task?.cancel?.(currentTaskId); } catch (e) { console.warn('Cancel failed:', e); }
  }, [currentTaskId]);

  const send = useCallback(async (prompt: string) => {
    const oz = (window as any).ozone;
    if (!prompt.trim() || !oz?.orchestrate) return;
    const key = contextKey;
    const submitTs = Date.now();

    setTranscriptFor(key, prev => [...prev, { id: submitTs, role: 'user', content: prompt, timestamp: submitTs }]);
    const placeholderId = submitTs + 1;
    setTranscriptFor(key, prev => [...prev, { id: placeholderId, role: 'assistant', content: '⏳ Thinking...', timestamp: submitTs }]);

    setIsRunning(true);
    setAwaiting(null);
    setCurrentTaskId(null);
    setStepInfo(null);
    setElapsedSeconds(0);
    stopTick();
    tickRef.current = setInterval(() => setElapsedSeconds(s => s + 1), 1000);

    // Baseline: ids that existed before this submit. Only new ones are ours.
    const baseline = new Set<number>();
    try {
      const l0 = await oz.task?.list?.();
      for (const t of l0?.tasks ?? []) if (typeof t.task_id === 'number') baseline.add(t.task_id);
    } catch { /* baseline best-effort; created_at check below backs it up */ }

    let foundTaskId: number | null = null;
    let settled = false;
    const discover = async () => {
      if (foundTaskId) {
        try {
          const d = await oz.task?.status?.(foundTaskId);
          const steps = d?.steps ?? [];
          if (steps.length > 0) {
            setStepInfo({
              completed: steps.filter((s: any) => s.status === 'completed').length,
              total: steps.length,
              lastAction: steps[steps.length - 1]?.action,
            });
          }
          if (d) setAwaiting(awaitingKindOf(d.status));
        } catch { /* transient */ }
        return;
      }
      try {
        const list = await oz.task?.list?.();
        const fresh = (list?.tasks ?? [])
          .filter((t: any) => !baseline.has(t.task_id))
          .filter((t: any) => {
            const c = Number(t.created_at ?? 0);
            if (!c) return true;
            const ms = c < 1e12 ? c * 1000 : c; // seconds or ms
            return ms >= submitTs - 2000;
          })
          .sort((a: any, b: any) => (b.created_at ?? 0) - (a.created_at ?? 0));
        if (fresh.length > 0) {
          foundTaskId = fresh[0].task_id;
          setCurrentTaskId(foundTaskId);
        }
      } catch { /* transient */ }
    };
    const discoveryTimer = setInterval(() => { if (!settled) void discover(); }, DISCOVERY_INTERVAL_MS);

    try {
      const { user_id, device_id } = authIds();
      const result = await oz.orchestrate({
        prompt,
        project_id: projectId,
        workspace_id: workspaceId,
        user_id,
        device_id,
        consciousness_enabled: consciousnessEnabled,
        token_budget: 100000,
        model_config: modelConfig,
      });
      settled = true;
      clearInterval(discoveryTimer);

      if (result?.awaiting === true) {
        // Durable awaiting: the host detached the walk and is waiting on a
        // meta/consciousness judgment (or the operator). Not an error.
        const tid = Number(result.task_id ?? foundTaskId ?? 0);
        if (tid) {
          setCurrentTaskId(tid);
          const finalTask = await recoverTaskResult(tid, placeholderId, key);
          if (finalTask) await onResultRef.current?.({ ...finalTask, response: finalTask.response, success: !finalTask.error });
        } else {
          setTranscriptFor(key, prev => prev.map(e => e.id === placeholderId
            ? { ...e, content: '⏳ Waiting on a consciousness/meta judgment — the host did not return a task id; check the Tasks view.' }
            : e));
        }
      } else {
        const pauseNotice = parsePause(result?.error, result?.task_id ?? foundTaskId);
        if (pauseNotice && pauseNotice.taskId) setPaused({ taskId: pauseNotice.taskId, step: pauseNotice.step, error: pauseNotice.error });
        setTranscriptFor(key, prev => prev.map(entry =>
          entry.id === placeholderId
            ? applyResultToEntry(entry, {
                response: result.response,
                error: result.error,
                modelUsed: result.model_used,
                stageCount: Array.isArray(result.stages_completed) ? result.stages_completed.length : undefined,
                thinkingLog: result.thinking_log,
                amtSummary: result.amt_summary,
                attemptTrail: result.attempt_trail,
                refusals: result.refusals,
              })
            : entry
        ));
        await onResultRef.current?.(result);
      }
    } catch (e: any) {
      settled = true;
      clearInterval(discoveryTimer);
      const msg = String(e?.message ?? '');
      const isTimeout = msg.toLowerCase().includes('timeout');
      if (isTimeout) {
        // Client gave up waiting; the server walk is detached and may still be
        // running. Keep discovering (≤60 s) the task this submit created.
        const until = Date.now() + DISCOVERY_RETRY_MS;
        while (!foundTaskId && Date.now() < until) {
          await discover();
          if (!foundTaskId) await new Promise(r => setTimeout(r, DISCOVERY_INTERVAL_MS * 2));
        }
        if (foundTaskId) {
          setTranscriptFor(key, prev => prev.map(en => en.id === placeholderId
            ? { ...en, content: `⏳ Still working — recovering the real result (task ${foundTaskId})...` }
            : en));
          const finalTask = await recoverTaskResult(foundTaskId, placeholderId, key);
          if (finalTask) await onResultRef.current?.({ ...finalTask, success: !finalTask.error });
        } else {
          setTranscriptFor(key, prev => prev.map(en => en.id === placeholderId
            ? { ...en, content: 'The client stopped waiting and no task record was found for this request (it may not have been created). Check the Tasks view before retrying.' }
            : en));
        }
      } else {
        setTranscriptFor(key, prev => prev.map(en => en.id === placeholderId
          ? { ...en, content: `Error: ${msg || 'Request failed'}`, emotion: 'concerned' }
          : en));
      }
    } finally {
      settled = true;
      clearInterval(discoveryTimer);
      stopTick();
      setIsRunning(false);
      setCurrentTaskId(null);
      setStepInfo(null);
      setAwaiting(null);
    }
  }, [contextKey, projectId, workspaceId, consciousnessEnabled, modelConfig, recoverTaskResult, setTranscriptFor, stopTick]);

  /** Resume the failure-paused task from exactly the step it stopped at, then follow it. */
  const resumePaused = useCallback(async () => {
    const p = paused;
    if (!p) return;
    const key = contextKey;
    const ts = Date.now();
    const placeholderId = ts;
    setTranscriptFor(key, prev => [...prev, { id: placeholderId, role: 'assistant', content: `⏳ Resuming task ${p.taskId} from step ${p.step}…`, timestamp: ts }]);
    setPaused(null);
    setIsRunning(true);
    setCurrentTaskId(p.taskId);
    setElapsedSeconds(0);
    stopTick();
    tickRef.current = setInterval(() => setElapsedSeconds(s => s + 1), 1000);
    try {
      const r = await resumeRun(p.taskId);
      if (!r?.success) {
        setPaused(p);
        setTranscriptFor(key, prev => prev.map(e => (e.id === placeholderId ? { ...e, content: `Could not resume task ${p.taskId}: ${r?.error ?? 'host refused'}` } : e)));
        return;
      }
      const finalTask = await recoverTaskResult(p.taskId, placeholderId, key);
      if (finalTask && String(finalTask.status ?? '').toLowerCase() !== 'paused') {
        await onResultRef.current?.({ ...finalTask, success: !finalTask.error });
      }
    } catch (e: any) {
      setPaused(p);
      setTranscriptFor(key, prev => prev.map(en => (en.id === placeholderId ? { ...en, content: `Resume failed: ${e?.message ?? 'request failed'}` } : en)));
    } finally {
      stopTick();
      setIsRunning(false);
      setCurrentTaskId(null);
      setStepInfo(null);
      setAwaiting(null);
    }
  }, [paused, contextKey, recoverTaskResult, setTranscriptFor, stopTick]);

  return {
    paused,
    resumePaused,
    transcript,
    setTranscript,
    isRunning,
    elapsedSeconds,
    stepInfo,
    awaiting,
    currentTaskId,
    send,
    stop,
  };
}

export { isLiveTaskStatus };
