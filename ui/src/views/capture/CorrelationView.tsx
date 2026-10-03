/**
 * E3 — Raw Thought ↔ Graph correlation view.
 *
 * Groups the real S11 capture rows (zero_shot_calls.jsonl, via the B5 route
 * `GET /capture/zero-shot-calls`) by their correlation keys
 * `(amt_container_id, blueprint_id, project_id)` and resolves each key to the
 * real ZSEI container it names. This is the first consumer of those keys.
 *
 * Real sources (verified against the live host, 2026-09-24):
 *  - Row shape/keys: `capture_zero_shot_call`, src/orchestrator/mod.rs:2592-2638
 *    — the keys are copied from the per-request `OrchestrationState`
 *    (`state.amt_container_id`, `state.blueprint_id`, `state.request.project_id`).
 *  - Why keys are null: `amt_container_id` is assigned only when `build_amt`
 *    persists the finished AMT (src/orchestrator/amt.rs:60-65); the three AMT
 *    call sites (amt_intent_extraction / amt_branch_generation /
 *    amt_detail_extraction, amt.rs:1330/1494/1678) run inside
 *    `build_amt_layer_by_layer`, BEFORE that assignment, so they are always
 *    captured with a null AMT id. `blueprint_id` is assigned only after stage 3
 *    matches (>=95%) or creates a blueprint (stages.rs:340 / :865); the
 *    `blueprint_assignment` call (stages.rs:536) precedes both, and if every
 *    model fails no blueprint is ever created. `project_id` is whatever id the
 *    caller put in the request — it is NOT validated to be a Project.
 *  - Container resolution: `ZSEIQuery::GetContainer` over POST /zsei/query.
 *    NOTE the host wraps every reply as `{success, result, error}` and the
 *    Electron bridge passes that through untouched, so results are unwrapped
 *    locally here (`unwrapZsei`) instead of relying on a shared reader.
 *
 * Doctrine: real data only. Every current row is a failure (OpenRouter credits
 * exhausted) — that is the primary state this view is designed around, and a
 * group with no AMT/blueprint is a real, expected bucket, not a broken join.
 */
import React, { useCallback, useEffect, useMemo, useState } from "react";
import { fetchZeroShotCalls, ZeroShotCallRow } from "../../data/captureData";
import { zseiQuery } from "../../ozoneClient";
import { navigateTo } from "../../navigation";
import { CaptureSeverity, CaptureStatus, CaptureStatusBadge, classifyZeroShotCall } from "./captureStatus";

export type CorrelationViewProps = { projectId: number | null };

const PAGE = 500;
const MAX_ROWS = 5000;

const C_TEXT = "var(--color-text)";
const C_BODY = "var(--color-text-secondary)";
const C_MUTED = "var(--color-text-muted)";
const C_BORDER = "var(--color-border-faint)";
const C_PANEL = "var(--color-bg)";
const C_WARN = "#e8c14f";
const C_BAD = "#ff8a8a";

// ── ZSEI helpers ─────────────────────────────────────────────────────────

/** The host replies `{success, result, error}`; the bridge does not unwrap it. Tolerate a bare result too. */
function unwrapZsei<T>(reply: unknown): T {
  if (reply && typeof reply === "object" && "success" in reply && ("result" in reply || "error" in reply)) {
    const r = reply as { success: boolean; result?: unknown; error?: unknown };
    if (r.success === false) throw new Error(typeof r.error === "string" && r.error ? r.error : "query failed");
    return r.result as T;
  }
  return reply as T;
}

interface ResolvedContainer {
  id: number;
  containerType: string;
  name: string | null;
  parentId: number | null;
  keywords: string[];
  objectStorePath: string | null;
  isMainAmt: boolean;
  forkOf: number | null;
  forkOfSource: "edge" | "keyword" | null;
}
type Resolution = { kind: "ok"; c: ResolvedContainer } | { kind: "error"; message: string };

async function resolveContainer(id: number): Promise<Resolution> {
  try {
    const res = unwrapZsei<any>(await zseiQuery<unknown>({ GetContainer: { container_id: id } }));
    const c = res?.Container;
    if (!c) return { kind: "error", message: "no container in reply" };
    const keywords: string[] = c.local_state?.context?.keywords ?? [];
    const relations: any[] = c.local_state?.context?.relationships ?? [];
    const edge = relations.find((r) => r?.relation_type === "ForkOf");
    const kw = keywords.find((k) => k.startsWith("amt-fork-of:"));
    const kwId = kw ? Number(kw.slice("amt-fork-of:".length)) : NaN;
    return {
      kind: "ok",
      c: {
        id,
        containerType: c.local_state?.metadata?.container_type ?? "?",
        name: c.local_state?.metadata?.name ?? null,
        parentId: c.global_state?.parent_id ?? null,
        keywords,
        objectStorePath: c.local_state?.storage?.object_store_path ?? null,
        isMainAmt: keywords.includes("amt-main"),
        forkOf: edge ? edge.target_id : Number.isFinite(kwId) ? kwId : null,
        forkOfSource: edge ? "edge" : Number.isFinite(kwId) ? "keyword" : null,
      },
    };
  } catch (e) {
    return { kind: "error", message: e instanceof Error ? e.message : String(e) };
  }
}

// ── Grouping ─────────────────────────────────────────────────────────────

interface Group {
  key: string;
  amt: number | null;
  bp: number | null;
  proj: number | null;
  rows: ZeroShotCallRow[];
  ok: number;
  failed: number;
  fallback: number;
  retries: number;
  tokens: number;
  callSites: Map<string, number>;
  models: Map<string, number>;
  firstTs: string;
  lastTs: string;
  status: CaptureStatus;
}

// Worst-first, so a group is never presented as healthier than its worst row.
const SEVERITY_RANK: Record<CaptureSeverity, number> = {
  failure: 0,
  "review-pending": 1,
  fallback: 2,
  unclassified: 3,
  success: 4,
};

const NO_MODEL = "(no model answered)";

function buildGroups(rows: ZeroShotCallRow[]): Group[] {
  const byKey = new Map<string, Group>();
  for (const r of rows) {
    const key = `${r.amt_container_id ?? "-"}|${r.blueprint_id ?? "-"}|${r.project_id ?? "-"}`;
    let g = byKey.get(key);
    if (!g) {
      g = {
        key,
        amt: r.amt_container_id,
        bp: r.blueprint_id,
        proj: r.project_id,
        rows: [],
        ok: 0,
        failed: 0,
        fallback: 0,
        retries: 0,
        tokens: 0,
        callSites: new Map(),
        models: new Map(),
        firstTs: r.ts,
        lastTs: r.ts,
        status: { severity: "success", label: "", reasons: [] },
      };
      byKey.set(key, g);
    }
    g.rows.push(r);
    if (r.success) g.ok++;
    else g.failed++;
    if (r.used_fallback) g.fallback++;
    g.retries += r.retry_count;
    g.tokens += r.tokens_used;
    g.callSites.set(r.call_site, (g.callSites.get(r.call_site) ?? 0) + 1);
    const m = r.model_used || NO_MODEL;
    g.models.set(m, (g.models.get(m) ?? 0) + 1);
    if (r.ts < g.firstTs) g.firstTs = r.ts;
    if (r.ts > g.lastTs) g.lastTs = r.ts;
  }
  const groups = Array.from(byKey.values());
  for (const g of groups) {
    g.rows.sort((a, b) => (a.ts < b.ts ? 1 : -1)); // newest first
    let worst: CaptureStatus | null = null;
    for (const r of g.rows) {
      const s = classifyZeroShotCall(r);
      if (!worst || SEVERITY_RANK[s.severity] < SEVERITY_RANK[worst.severity]) worst = s;
    }
    if (worst) g.status = worst;
  }
  // Correlated groups first, then the uncorrelated bucket; newest activity first within each.
  groups.sort((a, b) => {
    const ac = a.amt !== null || a.bp !== null ? 0 : 1;
    const bc = b.amt !== null || b.bp !== null ? 0 : 1;
    if (ac !== bc) return ac - bc;
    return a.lastTs < b.lastTs ? 1 : -1;
  });
  return groups;
}

// ── Small presentational bits ────────────────────────────────────────────

const wrap: React.CSSProperties = { overflowWrap: "anywhere", wordBreak: "break-word", minWidth: 0 };

function fmtTs(ts: string): string {
  const d = new Date(ts);
  return Number.isNaN(d.getTime()) ? ts : d.toLocaleString();
}
function counts(m: Map<string, number>): string {
  return Array.from(m.entries())
    .sort((a, b) => b[1] - a[1])
    .map(([k, n]) => `${k} ×${n}`)
    .join(", ");
}

const Row: React.FC<{ label: string; children: React.ReactNode }> = ({ label, children }) => (
  <div style={{ ...wrap, marginBottom: 2 }}>
    <span style={{ color: C_MUTED }}>{label}: </span>
    <span style={{ color: C_BODY }}>{children}</span>
  </div>
);

const ContainerLine: React.FC<{ label: string; id: number; res: Resolution | undefined; pending: boolean }> = ({
  label,
  id,
  res,
  pending,
}) => {
  let body: React.ReactNode;
  if (!res) body = <span style={{ color: C_MUTED }}>{pending ? "resolving…" : "not resolved"}</span>;
  else if (res.kind === "error") body = <span style={{ color: C_BAD }}>does not resolve ({res.message})</span>;
  else {
    const c = res.c;
    const bits: string[] = [c.containerType];
    if (c.containerType === "Derived" && c.keywords.some((k) => k === "amt-main" || k.startsWith("amt-fork-of:"))) {
      bits.push(c.isMainAmt ? "AMT main" : "AMT fork");
    }
    body = (
      <>
        <span style={{ color: C_TEXT }}>{c.name ?? "(unnamed)"}</span>{" "}
        <span style={{ color: C_MUTED }}>({bits.join(" · ")}{c.parentId !== null ? `, parent ${c.parentId}` : ""})</span>
        {c.forkOf !== null && (
          <div style={{ color: C_MUTED, fontSize: 11 }}>
            fork of container {c.forkOf} via {c.forkOfSource === "edge" ? "a real ForkOf edge" : "the legacy amt-fork-of keyword"}
          </div>
        )}
        {c.objectStorePath && <div style={{ color: C_MUTED, fontSize: 11 }}>content: {c.objectStorePath}</div>}
      </>
    );
  }
  return (
    <div style={{ ...wrap, marginBottom: 3 }}>
      <span style={{ color: C_MUTED }}>{label} {id}: </span>
      {body}{" "}
      {(!res || res.kind === "ok") && (
        <button
          onClick={() => navigateTo({ kind: "container", containerId: id })}
          title="Open this container in the Context Viewer (available once cross-view navigation is wired)"
          style={{ background: "none", border: `1px solid ${C_BORDER}`, color: C_MUTED, borderRadius: 999, padding: "0 8px", fontSize: 11, cursor: "pointer" }}
        >
          open ↗
        </button>
      )}
    </div>
  );
};

// ── View ─────────────────────────────────────────────────────────────────

export const CorrelationView: React.FC<CorrelationViewProps> = ({ projectId }) => {
  const [rows, setRows] = useState<ZeroShotCallRow[]>([]);
  const [capped, setCapped] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [onlySelected, setOnlySelected] = useState(false);
  const [touched, setTouched] = useState(false);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [resolved, setResolved] = useState<Record<number, Resolution>>({});
  const [resolving, setResolving] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const all: ZeroShotCallRow[] = [];
      let offset = 0;
      let hitCap = false;
      for (;;) {
        const page = await fetchZeroShotCalls({ offset, limit: PAGE });
        all.push(...page.rows);
        if (page.rows.length < PAGE) break;
        offset += PAGE;
        if (all.length >= MAX_ROWS) {
          hitCap = true;
          break;
        }
      }
      setRows(all);
      setCapped(hitCap);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setRows([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  // Default to the selected project only when it actually has captured calls; never silently show an empty view.
  useEffect(() => {
    if (!touched) setOnlySelected(projectId !== null && rows.some((r) => r.project_id === projectId));
  }, [rows, projectId, touched]);

  const scopedRows = useMemo(
    () => (onlySelected && projectId !== null ? rows.filter((r) => r.project_id === projectId) : rows),
    [rows, onlySelected, projectId],
  );
  const groups = useMemo(() => buildGroups(scopedRows), [scopedRows]);

  // Resolve every distinct real container id named by the visible groups (AMT, blueprint, project-key container).
  const idsToResolve = useMemo(() => {
    const s = new Set<number>();
    for (const g of groups) {
      if (g.amt !== null) s.add(g.amt);
      if (g.bp !== null) s.add(g.bp);
      if (g.proj !== null) s.add(g.proj);
    }
    return Array.from(s);
  }, [groups]);

  useEffect(() => {
    const missing = idsToResolve.filter((id) => !(id in resolved));
    if (missing.length === 0) return;
    let cancelled = false;
    setResolving(true);
    Promise.all(missing.map(async (id) => [id, await resolveContainer(id)] as const)).then((pairs) => {
      if (cancelled) return;
      setResolved((prev) => {
        const next = { ...prev };
        for (const [id, r] of pairs) next[id] = r;
        return next;
      });
      setResolving(false);
    });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [idsToResolve]);

  const correlatedRows = scopedRows.filter((r) => r.amt_container_id !== null || r.blueprint_id !== null).length;
  const toggle = (key: string) =>
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });

  return (
    <div style={{ color: C_BODY, fontSize: 12.5 }}>
      <div style={{ display: "flex", gap: 14, alignItems: "center", flexWrap: "wrap", marginBottom: 8 }}>
        <span style={{ fontWeight: 700, color: C_TEXT }}>Raw thought ↔ graph correlation</span>
        <button
          onClick={load}
          disabled={loading}
          style={{ background: "transparent", color: C_MUTED, border: `1px solid ${C_BORDER}`, borderRadius: 6, padding: "2px 10px", fontSize: 12, cursor: "pointer" }}
        >
          {loading ? "Loading…" : "Refresh"}
        </button>
        {projectId !== null && (
          <label style={{ display: "flex", alignItems: "center", gap: 5, cursor: "pointer", color: C_MUTED }}>
            <input
              type="checkbox"
              checked={onlySelected}
              style={{ margin: 0 }}
              onChange={() => {
                setTouched(true);
                setOnlySelected((v) => !v);
              }}
            />
            Only calls whose project_id is the selected project ({projectId})
          </label>
        )}
      </div>

      <div
        style={{
          ...wrap,
          border: `1px solid ${C_BORDER}`,
          background: C_PANEL,
          borderRadius: 8,
          padding: "8px 10px",
          marginBottom: 10,
          fontSize: 11.5,
          color: C_MUTED,
          lineHeight: 1.5,
        }}
      >
        <b style={{ color: C_BODY }}>Why some keys are null.</b> <code>amt_container_id</code> is only set once a request's
        AMT has been built, validated and persisted; the three AMT calls (<code>amt_intent_extraction</code>,{" "}
        <code>amt_branch_generation</code>, <code>amt_detail_extraction</code>) run before that, so they are always captured
        with no AMT id — even when they succeed. <code>blueprint_id</code> is only set after stage 3 matches or creates a
        blueprint, so the <code>blueprint_assignment</code> call itself precedes it, and if every model fails no blueprint
        is ever created. <code>project_id</code> is whatever id the caller sent; it is not validated to be a Project. An
        "uncorrelated" group is therefore a real, expected bucket, not a broken join.
      </div>

      {error && <div style={{ color: C_BAD, marginBottom: 8 }}>Error loading capture store: {error}</div>}
      {!error && loading && rows.length === 0 && <div style={{ color: C_MUTED }}>Loading captured calls…</div>}

      {!error && !loading && rows.length === 0 && (
        <div style={{ color: C_MUTED }}>
          No zero-shot calls have been captured yet (zero_shot_calls.jsonl is empty or absent) — nothing to correlate.
        </div>
      )}

      {rows.length > 0 && (
        <div style={{ color: C_MUTED, marginBottom: 8 }}>
          {scopedRows.length} call{scopedRows.length === 1 ? "" : "s"} shown
          {onlySelected ? ` (of ${rows.length} captured overall)` : ""} · {correlatedRows} correlated to an AMT or blueprint ·{" "}
          {scopedRows.length - correlatedRows} with neither · {groups.length} group{groups.length === 1 ? "" : "s"}
          {capped && (
            <span style={{ color: C_WARN }}>
              {" "}
              — capped: only the oldest {MAX_ROWS} rows were loaded, newer calls are not included
            </span>
          )}
          {resolving && " · resolving containers…"}
        </div>
      )}

      {rows.length > 0 && scopedRows.length === 0 && (
        <div style={{ color: C_WARN, marginBottom: 8 }}>
          No captured call has project_id {projectId}. Untick the filter to see all {rows.length} captured calls.
        </div>
      )}

      {groups.map((g) => {
        const correlated = g.amt !== null || g.bp !== null;
        const open = expanded.has(g.key);
        return (
          <div key={g.key} style={{ border: `1px solid ${C_BORDER}`, borderRadius: 8, marginBottom: 8, background: C_PANEL }}>
            <div
              onClick={() => toggle(g.key)}
              style={{ display: "flex", gap: 10, alignItems: "center", flexWrap: "wrap", padding: "8px 10px", cursor: "pointer" }}
            >
              <span style={{ color: C_MUTED }}>{open ? "▾" : "▸"}</span>
              <span style={{ ...wrap, fontWeight: 600, color: C_TEXT }}>
                {correlated
                  ? [g.amt !== null ? `AMT ${g.amt}` : null, g.bp !== null ? `blueprint ${g.bp}` : null].filter(Boolean).join(" · ")
                  : "Uncorrelated (no AMT/blueprint yet)"}
                {g.proj !== null ? ` · project_id ${g.proj}` : " · no project_id"}
              </span>
              <CaptureStatusBadge status={g.status} />
              <span style={{ marginLeft: "auto", color: C_MUTED }}>
                {g.rows.length} call{g.rows.length === 1 ? "" : "s"} · {g.ok} ok · <span style={{ color: g.failed ? C_BAD : C_MUTED }}>{g.failed} failed</span>
              </span>
            </div>

            <div style={{ padding: "0 10px 8px 26px", fontSize: 12 }}>
              {g.amt !== null && <ContainerLine label="AMT container" id={g.amt} res={resolved[g.amt]} pending={resolving} />}
              {g.bp !== null && <ContainerLine label="Blueprint" id={g.bp} res={resolved[g.bp]} pending={resolving} />}
              {g.proj !== null && <ContainerLine label="project_id container" id={g.proj} res={resolved[g.proj]} pending={resolving} />}
              <Row label="Call sites">{counts(g.callSites)}</Row>
              <Row label="Models">{counts(g.models)}</Row>
              <Row label="Fallback used">{g.fallback} of {g.rows.length}</Row>
              <Row label="Retries / tokens">{g.retries} retries · {g.tokens} tokens</Row>
              <Row label="Time span">
                {fmtTs(g.firstTs)}
                {g.firstTs !== g.lastTs ? ` → ${fmtTs(g.lastTs)}` : ""}
              </Row>
              {g.status.reasons.length > 0 && <Row label="Status reasons">{g.status.reasons.join("; ")}</Row>}
            </div>

            {open && (
              <div style={{ borderTop: `1px solid ${C_BORDER}`, padding: "6px 10px 8px 26px" }}>
                {g.rows.map((r, i) => {
                  const st = classifyZeroShotCall(r);
                  return (
                    <div key={`${r.ts}-${i}`} style={{ ...wrap, padding: "6px 0", borderBottom: i < g.rows.length - 1 ? `1px dashed ${C_BORDER}` : "none" }}>
                      <div style={{ display: "flex", gap: 10, flexWrap: "wrap", alignItems: "center" }}>
                        <span style={{ color: C_MUTED }}>{fmtTs(r.ts)}</span>
                        <span style={{ color: C_TEXT, fontWeight: 600 }}>{r.call_site}</span>
                        <span style={{ color: r.model_used ? C_BODY : C_MUTED }}>{r.model_used || NO_MODEL}</span>
                        <CaptureStatusBadge status={st} />
                        <span style={{ color: C_MUTED }}>
                          {r.retry_count} retr{r.retry_count === 1 ? "y" : "ies"} · {r.tokens_used} tok
                          {r.used_fallback ? " · fallback" : ""}
                        </span>
                      </div>
                      <pre
                        style={{
                          ...wrap,
                          whiteSpace: "pre-wrap",
                          margin: "4px 0 0",
                          maxHeight: 140,
                          overflowY: "auto",
                          background: "#070b13",
                          border: `1px solid ${C_BORDER}`,
                          borderRadius: 6,
                          padding: "6px 8px",
                          fontSize: 11.5,
                          color: r.success ? C_BODY : C_BAD,
                          fontFamily: "inherit",
                        }}
                      >
                        {r.response_preview || "(empty response)"}
                      </pre>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
};

export default CorrelationView;
