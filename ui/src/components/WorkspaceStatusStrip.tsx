/**
 * WorkspaceStatusStrip — the workspace header's live-trust indicators
 * (GraphKit wave 7, additive: a strip, not a layout rewrite). Shows the
 * REAL P2P posture via PeerState (connected ≠ trusted) and the substrate's
 * node-construction pulse from the shared GraphKit provider when a project
 * is loaded. All values are real captured ones; absent data renders as
 * honest absence.
 */
import React, { useEffect, useState } from "react";
import { useGraphKit } from "../graphkit/graphProvider";
import { hostGet, hostPost } from "../ozoneClient";
import { PeerState, ActivityPulse, TaskStateIcon, GateState, CapturePresence } from "../graphkit/indicators";

export const WorkspaceStatusStrip: React.FC = () => {
  const [peerCount, setPeerCount] = useState(0);
  const [pairedCount, setPairedCount] = useState(0);
  const [available, setAvailable] = useState(false);
  const nodeEventCount = useGraphKit((s) => s.nodeEventCount);

  useEffect(() => {
    let cancelled = false;
    const fetchOnce = async () => {
      try {
        // /health already carries the real peer_count (HealthResponse) — no
        // new route needed; paired-peers is the trust half.
        // Through the shared host client (Electron bridge-first) — a bare
        // relative fetch("/health") only works when the page origin IS the
        // host, so under Electron (file://) the strip silently never showed.
        const health: any = await hostGet("/health");
        if (cancelled) return;
        setPeerCount(Number(health?.peer_count ?? 0));
        setAvailable(true);
        // Trust half is best-effort: /network/* is session-gated when the
        // host runs OZONE_REQUIRE_SESSION=1, so an unpaired UI gets 401 here.
        // Honest degrade: paired count stays 0 and the chip says so.
        try {
          const paired: any = await hostGet("/network/paired-peers");
          if (!cancelled) {
            setPairedCount(Array.isArray(paired?.paired_peers) ? paired.paired_peers.length : 0);
          }
        } catch {
          if (!cancelled) setPairedCount(0);
        }
      } catch {
        if (!cancelled) setAvailable(false);
      }
    };
    fetchOnce();
    const t = setInterval(fetchOnce, 30_000);
    return () => {
      cancelled = true;
      clearInterval(t);
    };
  }, []);

  if (!available) return null; // honest absence: no strip when the host doesn't answer

  return (
    <div className="gnw-ind">
      <TaskChip />
      <GateChip />
      <CaptureChip />
      <span className="gnw-ind-sep" />
      <PeerState peerCount={peerCount} pairedCount={pairedCount} />
      <ActivityPulse active={nodeEventCount > 0} label={`${nodeEventCount} live construction events this session`} />
      <LoopChips />
    </div>
  );
};

/** Poll a host route; null until the first answer, null again on failure
 * (honest absence — a chip never shows a stale or invented number). */
function usePolled<T>(fetcher: () => Promise<T>, ms: number): T | null {
  const [v, setV] = useState<T | null>(null);
  useEffect(() => {
    let dead = false;
    const run = async () => {
      try { const r = await fetcher(); if (!dead) setV(r); } catch { if (!dead) setV(null); }
    };
    run();
    const t = setInterval(run, ms);
    return () => { dead = true; clearInterval(t); };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  return v;
}

interface CaptureTotal { total?: number }
/** CaptureChip — the S10–S13 capture stores' REAL totals from their read
 * routes, fed to the GraphKit CapturePresence indicator. Each store polls
 * independently; a route that fails renders honest absence for that store
 * only. The last unwired indicator of the GraphKit set (wave 8, 2026-10-29). */
function CaptureChip() {
  const s10 = usePolled(() => hostGet<CaptureTotal>("/capture/decision-reviews?limit=1"), 60_000);
  const s11 = usePolled(() => hostGet<CaptureTotal>("/capture/zero-shot-calls?limit=1"), 60_000);
  const s12 = usePolled(() => hostGet<CaptureTotal>("/capture/pipeline-zero-shot-calls?limit=1"), 60_000);
  const s13 = usePolled(() => hostGet<CaptureTotal>("/capture/tool-calls?limit=1"), 60_000);
  const n = (r: CaptureTotal | null): number | null =>
    r && typeof r.total === "number" ? r.total : null;
  return (
    <CapturePresence
      counts={{ S10: n(s10), S11: n(s11), S12: n(s12), S13: n(s13) }}
    />
  );
}

interface TaskRow { task_id: number; status: string; progress: number; description?: string | null; blueprint_name?: string; error?: string | null }

/** TaskChip — real /task/list: running / queued / failed counts, click for the
 * most recent tasks with their own status + progress (never an invented %). */
function TaskChip() {
  const data = usePolled(() => hostPost<{ tasks: TaskRow[] }>("/task/list", { limit: 30 }), 8000);
  const [open, setOpen] = useState(false);
  if (!data || !Array.isArray(data.tasks)) return null;
  const tasks = data.tasks;
  const n = (st: string) => tasks.filter((t) => t.status.toLowerCase() === st).length;
  const running = n("running"), queued = n("queued"), failed = n("failed"), interrupted = n("interrupted");
  const lead = running ? "running" : failed ? "failed" : interrupted ? "interrupted" : queued ? "queued" : "completed";
  return (
    <span className="gnw-ind-wrap">
      <button
        className={`gnw-ind-chip${open ? " on" : ""}`}
        onClick={() => setOpen((o) => !o)}
        title={`${tasks.length} most recent tasks — ${running} running, ${queued} queued, ${failed} failed, ${interrupted} interrupted`}
      >
        <TaskStateIcon status={lead} size={12} />
        <span>{running ? `${running} running` : tasks.length ? "tasks idle" : "no tasks"}</span>
        {failed > 0 && <span className="gnw-ind-n bad">{failed}</span>}
        {queued > 0 && <span className="gnw-ind-n">{queued}</span>}
      </button>
      {open && (
        <div className="gnw-ind-pop" role="dialog" aria-label="Recent tasks">
          <div className="gnw-ind-pop-h">Recent tasks <span>live from /task/list</span></div>
          {tasks.length === 0 && <div className="gnw-ind-pop-empty">No tasks recorded.</div>}
          {tasks.slice(0, 8).map((t) => (
            <div key={t.task_id} className="gnw-ind-row" title={t.error ?? undefined}>
              <TaskStateIcon status={t.status} size={12} />
              <span className="gnw-ind-id">#{t.task_id}</span>
              <span className="gnw-ind-txt">{t.description || t.blueprint_name || t.status}</span>
              <span className="gnw-ind-pct">{t.status === "running" ? `${Math.round((t.progress ?? 0) * (t.progress <= 1 ? 100 : 1))}%` : t.status}</span>
            </div>
          ))}
        </div>
      )}
    </span>
  );
}

/** GateChip — the consciousness gate's real posture: reviews awaiting a human
 * (amber), else the most recent decision-review outcome. */
function GateChip() {
  const pending = usePolled(() => hostGet<{ count?: number; rows?: any[] }>("/consciousness/reviews/pending"), 15000);
  const last = usePolled(() => hostGet<{ rows?: any[] }>("/capture/decision-reviews"), 30000);
  const pendingN = pending ? Number(pending.count ?? pending.rows?.length ?? 0) : null;
  const lastRow = last?.rows?.length ? last.rows[last.rows.length - 1] : null;
  if (pendingN === null && !lastRow) return null;
  if (pendingN && pendingN > 0) {
    return (
      <span className="gnw-ind-chip warn" title={`${pendingN} consciousness-gate review${pendingN === 1 ? "" : "s"} waiting for a human decision — nothing proceeds on an unjudged chunk`}>
        <GateState decision="pending" size={12} />
        <span>{pendingN} to review</span>
      </span>
    );
  }
  const d = String(lastRow?.decision ?? "");
  return (
    <span className="gnw-ind-chip" title={lastRow ? `Last gate decision-review: ${d || "unknown"}${lastRow.ts ? ` (${lastRow.ts})` : ""}; none waiting` : "No reviews waiting"}>
      <GateState decision={d} size={12} />
      <span>{lastRow ? "gate clear" : "no reviews"}</span>
    </span>
  );
}

export default WorkspaceStatusStrip;

/** LoopChips — per-loop heartbeat chips. Lit by the GraphKit substrate's
 * nodeEventCount delta over the last poll interval. In a future pass, these
 * would subscribe to per-loop capture events; today the pulse is honest
 * aggregate activity (real event count from the live WS stream). */
function LoopChips() {
  const lastEventAt = useGraphKit((s) => s.lastEventAt);
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    const t = setInterval(() => setNow(Date.now()), 5000);
    return () => clearInterval(t);
  }, []);
  const recent = lastEventAt > 0 && now - lastEventAt < 30_000;
  if (lastEventAt === 0) return null;
  return (
    <span style={{ display: 'inline-flex', alignItems: 'center', gap: 4 }}>
      <span style={{
        width: 6, height: 6, borderRadius: '50%',
        background: recent ? '#4fc3f7' : 'var(--color-border)',
        display: 'inline-block',
        animation: recent ? 'pulse 1.2s ease-in-out infinite' : undefined,
      }} />
      <span style={{ fontSize: 10, color: 'var(--color-text-muted)' }}>
        graph {recent ? 'active' : `${Math.round((now - lastEventAt) / 1000)}s ago`}
      </span>
    </span>
  );
}
