/**
 * CallGraphView — the system's OWN call graphs, visible with no prompt, with a live overlay of what a walk actually ran (operator 2026-11-11).
 *
 * STATIC (always): GET /callgraph/system — the graphs the host materialises as graph containers at boot (and serves straight from the generated
 * asset, so this works before any prompt):
 *   walk   the orchestration stages in the order the walk calls them → the functions that make model calls → the labelled call sites
 *   loops  the background loops and their labelled model-call sites
 *   routes the call routes and the guardrail layers on each
 * LIVE (when a walk is picked): GET /callgraph/live — the executed calls (capture/call_spans.jsonl) and stage events (walk_metrics.jsonl) of one
 * trace. A call site that ran shows "ran ×N" (and "failed ×M" if any call failed); one that did not shows nothing — absence is not a verdict. Pick the
 * walk with `taskId`, or `follow` the most recent walk. A call row is written when the call finishes, so a call still in flight shows up in the
 * activity strip (pipeline progress), not here — said on screen, not hidden.
 *
 * REAL DATA ONLY: edges are present or absent (found in the source), calls ran or did not. Nothing is scored, ranked or given a confidence.
 * The static extraction is NAME-BASED: a call site no stage reaches by name is listed under "not reached by name from a stage" rather than dropped.
 */
import React, { useEffect, useMemo, useState } from "react";
import {
  fetchCallGraphLive,
  fetchCallGraphSystem,
  type CallGraphDoc,
  type CallGraphLiveResponse,
  type CallGraphNode,
  type CallGraphSystemResponse,
} from "../ozoneClient";

export interface CallGraphViewProps {
  /** overlay this task's walk (its record carries the trace that created it) */
  taskId?: number | null;
  /** overlay the most recent walk instead (used where no task is selected) */
  follow?: boolean;
  /** milliseconds between live refreshes while an overlay is requested (default 3000) */
  pollMs?: number;
  /** collapse the graphs behind a summary (for use beside another view) */
  compact?: boolean;
}

function where(n: CallGraphNode): string {
  if (!n.file) return "";
  return n.line ? `${n.file}:${n.line}` : n.file;
}

function groupBy<T>(items: T[], key: (t: T) => string): Map<string, T[]> {
  const m = new Map<string, T[]>();
  for (const it of items) {
    const k = key(it);
    if (!m.has(k)) m.set(k, []);
    m.get(k)!.push(it);
  }
  return m;
}

const RunBadge: React.FC<{ runs?: number; failed?: number; ms?: number | null }> = ({ runs, failed, ms }) => {
  if (!runs) return null;
  return (
    <>
      <span className="ocg-badge ocg-ran" title="this call site was executed in the overlaid walk">ran ×{runs}</span>
      {failed ? <span className="ocg-badge ocg-failed" title="calls of this site that failed">failed ×{failed}</span> : null}
      {typeof ms === "number" ? <span className="ocg-dim">last {ms} ms</span> : null}
    </>
  );
};

const SiteRow: React.FC<{ n: CallGraphNode; live: CallGraphLiveResponse | null }> = ({ n, live }) => {
  const run = live?.by_call_site?.[n.label];
  return (
    <li className={`ocg-site${run?.runs ? " ocg-site-ran" : ""}`}>
      <span className="ocg-kind">{n.kind === "vote_site" ? "vote" : "call"}</span>
      <span className="ocg-label omono">{n.label}</span>
      <RunBadge runs={run?.runs} failed={run?.failed} ms={run?.last_latency_ms} />
      {n.in_registry === false && n.kind === "call_site" && <span className="ocg-badge ocg-warn" title="the label does not appear in docs/ZERO_SHOT_CALL_REGISTRY.md">not in registry</span>}
      <span className="ocg-dim omono">{where(n)}</span>
    </li>
  );
};

const WalkGraph: React.FC<{ g: CallGraphDoc; live: CallGraphLiveResponse | null }> = ({ g, live }) => {
  const byId = useMemo(() => new Map(g.nodes.map((n) => [n.id, n])), [g]);
  const stages = useMemo(() => g.nodes.filter((n) => n.kind === "stage").sort((a, b) => (a.order ?? 0) - (b.order ?? 0)), [g]);
  const reachesOf = useMemo(() => groupBy(g.edges.filter((e) => e.kind === "reaches"), (e) => e.from), [g]);
  const sitesOfFn = useMemo(() => groupBy(g.edges.filter((e) => e.kind === "calls_model" || e.kind === "calls_vote"), (e) => e.from), [g]);
  const unattached = g.nodes.filter((n) => n.kind === "call_site" && n.attached === false);
  return (
    <div className="ocg-walk">
      {stages.map((st) => {
        const fns = (reachesOf.get(st.id) ?? []).map((e) => ({ e, fn: byId.get(e.to) })).filter((x) => !!x.fn) as { e: { via?: string[] }; fn: CallGraphNode }[];
        const siteNodes = fns.flatMap(({ fn }) => (sitesOfFn.get(fn.id) ?? []).map((se) => byId.get(se.to)).filter((x): x is CallGraphNode => !!x));
        const ran = siteNodes.reduce((n, s) => n + (live?.by_call_site?.[s.label]?.runs ? 1 : 0), 0);
        return (
          <details key={st.id} className={`ocg-stage${ran ? " ocg-stage-ran" : ""}`} open={!!ran || stages.length <= 4}>
            <summary>
              <span className="ocg-order">{st.order}</span>
              <span className="ocg-label omono">{st.label}</span>
              <span className="ocg-dim omono">{where(st)}</span>
              {siteNodes.length > 0 && <span className="ocg-dim">{siteNodes.length} model-call site{siteNodes.length === 1 ? "" : "s"}{live && ran ? ` · ${ran} ran` : ""}</span>}
              {siteNodes.length === 0 && <span className="ocg-dim">no model call reached by name</span>}
            </summary>
            {fns.map(({ e, fn }) => (
              <div key={fn.id} className="ocg-fn">
                <div>
                  <span className="ocg-kind">fn</span>
                  <span className="ocg-label omono">{fn.label}</span>
                  <span className="ocg-dim omono">{where(fn)}</span>
                  {e.via && e.via.length > 1 && <span className="ocg-dim"> via {e.via.join(" → ")}</span>}
                </div>
                <ul className="ocg-sites">
                  {(sitesOfFn.get(fn.id) ?? []).map((se) => {
                    const n = byId.get(se.to);
                    return n ? <SiteRow key={n.id} n={n} live={live} /> : null;
                  })}
                </ul>
              </div>
            ))}
          </details>
        );
      })}
      {unattached.length > 0 && (
        <details className="ocg-stage">
          <summary>
            <span className="ocg-label">not reached by name from a stage</span>
            <span className="ocg-dim">{unattached.length} call site{unattached.length === 1 ? "" : "s"} — reached through a closure, a trait object or an ambiguous name</span>
          </summary>
          <ul className="ocg-sites">{unattached.map((n) => <SiteRow key={n.id} n={n} live={live} />)}</ul>
        </details>
      )}
    </div>
  );
};

const LoopsGraph: React.FC<{ g: CallGraphDoc; live: CallGraphLiveResponse | null }> = ({ g, live }) => {
  const byId = useMemo(() => new Map(g.nodes.map((n) => [n.id, n])), [g]);
  const loops = g.nodes.filter((n) => n.kind === "loop");
  const sites = useMemo(() => groupBy(g.edges.filter((e) => e.kind === "calls_model"), (e) => e.from), [g]);
  return (
    <div>
      {loops.map((l) => (
        <details key={l.id} className="ocg-stage" open>
          <summary>
            <span className="ocg-label omono">{l.label}</span>
            <span className="ocg-dim omono">{l.file}</span>
          </summary>
          <ul className="ocg-sites">
            {(sites.get(l.id) ?? []).map((e) => {
              const n = byId.get(e.to);
              return n ? <SiteRow key={n.id} n={n} live={live} /> : null;
            })}
          </ul>
        </details>
      ))}
    </div>
  );
};

const RoutesGraph: React.FC<{ g: CallGraphDoc }> = ({ g }) => {
  const byId = useMemo(() => new Map(g.nodes.map((n) => [n.id, n])), [g]);
  const routes = g.nodes.filter((n) => n.kind === "route");
  const guards = useMemo(() => groupBy(g.edges.filter((e) => e.kind === "guarded_by"), (e) => e.from), [g]);
  return (
    <div>
      {routes.map((r) => (
        <div key={r.id} className="ocg-route">
          <span className="ocg-label">{r.label}</span>
          {(guards.get(r.id) ?? []).map((e) => {
            const gn = byId.get(e.to);
            return gn ? <span key={e.to} className="ocg-badge ocg-guard" title={gn.detail || gn.where || ""}>{gn.label}</span> : null;
          })}
          {(guards.get(r.id) ?? []).length === 0 && <span className="ocg-dim">no guardrail layer named in the registry row</span>}
          {r.status ? <span className="ocg-dim"> {r.status}</span> : null}
        </div>
      ))}
    </div>
  );
};

export const CallGraphView: React.FC<CallGraphViewProps> = ({ taskId, follow, pollMs = 3000, compact }) => {
  const [system, setSystem] = useState<CallGraphSystemResponse | null>(null);
  const [error, setError] = useState("");
  const [live, setLive] = useState<CallGraphLiveResponse | null>(null);
  const [tab, setTab] = useState("walk");

  useEffect(() => {
    let cancelled = false;
    fetchCallGraphSystem()
      .then((r) => { if (!cancelled) { setSystem(r); setError(""); } })
      .catch((e) => { if (!cancelled) setError(String(e?.message ?? e)); });
    return () => { cancelled = true; };
  }, []);

  const overlay = typeof taskId === "number" ? { taskId } : follow ? { latest: true } : null;
  const overlayKey = overlay ? JSON.stringify(overlay) : "";
  useEffect(() => {
    if (!overlay) { setLive(null); return; }
    let cancelled = false;
    const load = async (): Promise<void> => {
      try {
        const r = await fetchCallGraphLive(overlay);
        if (!cancelled && r && r.success) setLive(r);
      } catch {
        /* keep the last overlay; the static graph does not depend on it */
      }
    };
    load();
    const t = setInterval(load, pollMs);
    return () => { cancelled = true; clearInterval(t); };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [overlayKey, pollMs]);

  const graphs = system?.graphs ?? [];
  const current = graphs.find((g) => g.key === tab) ?? graphs[0];

  const body = (
    <div className="ocg">
      {error && <div className="ocg-note ocg-err">Could not load the call graphs: {error}</div>}
      {system && !system.available && <div className="ocg-note">{system.note ?? "The call graph asset is missing."}</div>}
      {graphs.length > 0 && (
        <>
          <div className="ocg-tabs" role="tablist">
            {graphs.map((g) => (
              <button key={g.key} type="button" role="tab" aria-selected={g.key === current?.key} className={`ocg-tab${g.key === current?.key ? " ocg-tab-on" : ""}`} onClick={() => setTab(g.key)}>
                {g.key}
              </button>
            ))}
          </div>
          {current && (
            <div className="ocg-title">
              {current.title}
              {current.container_id !== null ? <span className="ocg-dim"> · graph container #{current.container_id}</span> : <span className="ocg-dim"> · container not created yet (created at host boot)</span>}
            </div>
          )}
        </>
      )}
      {overlay && (
        <div className="ocg-live">
          {live?.trace_id ? (
            <>
              <span className="ocg-badge ocg-ran">live overlay</span>
              <span className="ocg-dim omono">trace {live.trace_id}</span>
              <span className="ocg-dim">{live.calls.length} call{live.calls.length === 1 ? "" : "s"} finished · {live.stages.length} stage event{live.stages.length === 1 ? "" : "s"}{live.unlabelled_calls ? ` · ${live.unlabelled_calls} unlabelled` : ""}</span>
              <div className="ocg-stages">
                {live.stages.map((s, i) => (
                  <span key={`${s.stage}:${i}`} className={`ocg-badge ${s.success === false ? "ocg-failed" : "ocg-ran"}`} title={s.summary ?? ""}>
                    {s.stage_name ?? `stage ${s.stage}`}{typeof s.duration_ms === "number" ? ` ${s.duration_ms} ms` : ""}
                  </span>
                ))}
              </div>
            </>
          ) : (
            <span className="ocg-dim">no walk trace to overlay yet — the static graph is shown. {live?.note ?? ""}</span>
          )}
          <div className="ocg-dim">a call row is written when the call finishes; calls still in flight show in the activity strip</div>
        </div>
      )}
      {current?.key === "walk" && <WalkGraph g={current} live={live} />}
      {current?.key === "loops" && <LoopsGraph g={current} live={live} />}
      {current?.key === "routes" && <RoutesGraph g={current} />}
      {current && !["walk", "loops", "routes"].includes(current.key) && (
        <div className="ocg-note">{current.nodes.length} nodes · {current.edges.length} edges (no dedicated renderer for “{current.key}”)</div>
      )}
      {system?.notes && system.notes.length > 0 && (
        <details className="ocg-notes">
          <summary>how this was extracted</summary>
          <ul>{system.notes.map((n, i) => <li key={i}>{n}</li>)}</ul>
          {system.generator && <div className="ocg-dim omono">{system.generator}</div>}
        </details>
      )}
    </div>
  );

  if (compact) {
    return (
      <details className="ocg-compact">
        <summary>Orchestration call graph{overlay ? " (live overlay)" : ""}</summary>
        {body}
      </details>
    );
  }
  return body;
};

export default CallGraphView;
