/**
 * E7 — Per-call-site reliability dashboard.
 *
 * Real aggregate statistics computed client-side from S11 (zero_shot_calls.jsonl, via
 * GET /capture/zero-shot-calls — B5), grouped by `call_site` then by `model_used`. This is the
 * "measured, not guessed" tuning data that Methodology 38
 * (assets/methodologies/method_38_tune_from_measured_metrics_never_preemptive_caps.json:
 * "Every optimization claim must carry a before/after pair from the same instrumented surface")
 * calls for — e.g. BitNet-vs-OpenRouter retry rates per call site.
 *
 * Real sources cited:
 *  - Row shape: `capture_zero_shot_call`, src/orchestrator/mod.rs:2592-2648 (12 fields).
 *  - The ONLY writer of S11 is `metered_execute_resilient` (src/orchestrator/mod.rs:2580 is the sole
 *    `capture_zero_shot_call` caller), so S11 covers exactly the call sites routed through it.
 *  - KNOWN_CALL_SITES below is the set of string literals passed as `call_site` to
 *    `metered_execute_resilient` in src/orchestrator/{amt,stages,graphs,response,mod}.rs (file:line
 *    each). It is a static snapshot of the source — a site seen in the store but absent here is
 *    flagged rather than hidden, so drift stays visible. Inventory context: docs/ZERO_SHOT_CALL_REGISTRY.md §1.
 *  - Not written to S11 by construction (per that registry §1): the main step-execution dispatch,
 *    `confirm_yes_no_orch`, AMT re-expansion (amt_loop.rs), the methodology meta-loop (meta_loop.rs).
 *
 * Honesty rules: n is shown beside every rate; groups with n < 5 are flagged (not hidden); a 0%
 * success rate on all-failure data is the correct measured output and is never smoothed. The store
 * does not ripple (plain file appends), so this view refreshes on demand only.
 */
import React, { useEffect, useMemo, useState } from "react";
import { fetchZeroShotCalls, ZeroShotCallRow } from "../../data/captureData";

export type ReliabilityDashboardProps = { projectId: number | null };

const PAGE_SIZE = 500;
const ROW_CAP = 5000;
const MIN_N = 5;

const KNOWN_CALL_SITES: { name: string; source: string }[] = [
  { name: "file_role_classification", source: "graphs.rs:508" },
  { name: "amt_branch_graph_native", source: "amt.rs:1015" },
  { name: "amt_intent_extraction", source: "amt.rs:1330" },
  { name: "amt_branch_generation", source: "amt.rs:1494" },
  { name: "amt_detail_extraction", source: "amt.rs:1678" },
  { name: "amt_cross_ref", source: "amt.rs:1957" },
  { name: "methodology_domain_id", source: "amt.rs:2732" },
  { name: "methodology_synthesis", source: "amt.rs:2782" },
  { name: "response_graph_render", source: "response.rs:308" },
  { name: "blueprint_assignment", source: "stages.rs:536" },
  { name: "zero_shot_simulation", source: "stages.rs:1124" },
  { name: "web_search_decompose", source: "stages.rs:1723" },
  { name: "context_compaction", source: "stages.rs:1972" },
  { name: "methodology_compliance_check", source: "stages.rs:2226" },
  { name: "on_step_complete_alignment", source: "mod.rs:2899" },
];
const KNOWN_NAMES = new Set(KNOWN_CALL_SITES.map((c) => c.name));

// ── Aggregation (pure) ────────────────────────────────────────────────────

export interface Stats {
  n: number;
  ok: number;
  retrySum: number;
  withRetry: number;
  fallback: number;
  tokens: number;
}
export interface SiteGroup {
  callSite: string;
  total: Stats;
  models: { model: string; stats: Stats }[];
}

const emptyStats = (): Stats => ({ n: 0, ok: 0, retrySum: 0, withRetry: 0, fallback: 0, tokens: 0 });
function addRow(s: Stats, r: ZeroShotCallRow): void {
  s.n += 1;
  if (r.success) s.ok += 1;
  const retries = r.retry_count ?? 0;
  s.retrySum += retries;
  if (retries > 0) s.withRetry += 1;
  if (r.used_fallback) s.fallback += 1;
  s.tokens += r.tokens_used ?? 0;
}

export function aggregateCalls(rows: ZeroShotCallRow[]): { total: Stats; groups: SiteGroup[] } {
  const total = emptyStats();
  const bySite = new Map<string, { total: Stats; models: Map<string, Stats> }>();
  for (const r of rows) {
    addRow(total, r);
    let g = bySite.get(r.call_site);
    if (!g) {
      g = { total: emptyStats(), models: new Map() };
      bySite.set(r.call_site, g);
    }
    addRow(g.total, r);
    const model = r.model_used ?? "";
    let m = g.models.get(model);
    if (!m) {
      m = emptyStats();
      g.models.set(model, m);
    }
    addRow(m, r);
  }
  const groups: SiteGroup[] = Array.from(bySite.entries()).map(([callSite, g]) => ({
    callSite,
    total: g.total,
    models: Array.from(g.models.entries()).map(([model, stats]) => ({ model, stats })),
  }));
  return { total, groups };
}

type SortKey = "call_site" | "n" | "success" | "retries" | "fallback" | "tokens";
function metric(s: Stats, key: Exclude<SortKey, "call_site">): number {
  switch (key) {
    case "n":
      return s.n;
    case "success":
      return s.n ? s.ok / s.n : 0;
    case "retries":
      return s.n ? s.retrySum / s.n : 0;
    case "fallback":
      return s.n ? s.fallback / s.n : 0;
    case "tokens":
      return s.tokens;
  }
}

// ── Fetching ──────────────────────────────────────────────────────────────

async function fetchAll(): Promise<{ rows: ZeroShotCallRow[]; capped: boolean }> {
  const rows: ZeroShotCallRow[] = [];
  let offset = 0;
  for (;;) {
    const page = await fetchZeroShotCalls({ offset, limit: PAGE_SIZE });
    rows.push(...page.rows);
    offset += page.rows.length;
    if (page.rows.length === 0 || page.rows.length < Math.min(PAGE_SIZE, page.limit)) {
      return { rows, capped: false };
    }
    if (rows.length >= ROW_CAP) {
      // Probe one more row so "capped" is only claimed when more really exist.
      const probe = await fetchZeroShotCalls({ offset, limit: 1 });
      return { rows, capped: probe.rows.length > 0 };
    }
  }
}

// ── Presentation ──────────────────────────────────────────────────────────

const C_TEXT = "#dfe7f2";
const C_BODY = "#c7d0dc";
const C_MUTED = "#8b98ab";
const C_BORDER = "#1e2836";
const C_WARN = "#e8c14f";
const C_OK = "#8fe38f";
const C_RETRY = "#ffb95f";
const C_FALLBACK = "#5fb3ff";

const pct = (x: number): string => `${(x * 100).toFixed(x === 0 || x === 1 ? 0 : 1)}%`;
const modelLabel = (m: string): string => (m === "" ? "(no model answered)" : m);

const Bar: React.FC<{ frac: number; color: string; label: string }> = ({ frac, color, label }) => (
  <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
    <div
      role="img"
      aria-label={label}
      style={{ width: 56, height: 6, background: C_BORDER, borderRadius: 3, overflow: "hidden", flexShrink: 0 }}
    >
      <div style={{ width: `${Math.max(0, Math.min(1, frac)) * 100}%`, height: "100%", background: color }} />
    </div>
    <span style={{ whiteSpace: "nowrap" }}>{label}</span>
  </div>
);

const SmallN: React.FC<{ n: number }> = ({ n }) =>
  n < MIN_N ? (
    <span
      title={`Only ${n} call${n === 1 ? "" : "s"} — fewer than ${MIN_N}, too few to conclude anything about a rate.`}
      style={{ marginLeft: 6, fontSize: 10.5, color: C_WARN, border: `1px solid ${C_WARN}`, borderRadius: 999, padding: "0 6px", whiteSpace: "nowrap" }}
    >
      n&lt;{MIN_N}: too few to conclude
    </span>
  ) : null;

const StatCells: React.FC<{ s: Stats; strong?: boolean }> = ({ s, strong }) => {
  const td: React.CSSProperties = { padding: "5px 10px", verticalAlign: "middle", fontWeight: strong ? 600 : 400 };
  const okFrac = s.n ? s.ok / s.n : 0;
  const retryFrac = s.n ? s.withRetry / s.n : 0;
  const fbFrac = s.n ? s.fallback / s.n : 0;
  return (
    <>
      <td style={td}>
        {s.n}
        <SmallN n={s.n} />
      </td>
      <td style={td}>
        <Bar frac={okFrac} color={C_OK} label={`${pct(okFrac)} (${s.ok}/${s.n})`} />
      </td>
      <td style={td}>
        <Bar
          frac={retryFrac}
          color={C_RETRY}
          label={`avg ${(s.n ? s.retrySum / s.n : 0).toFixed(2)} · ${pct(retryFrac)} retried (${s.withRetry}/${s.n})`}
        />
      </td>
      <td style={td}>
        <Bar frac={fbFrac} color={C_FALLBACK} label={`${pct(fbFrac)} (${s.fallback}/${s.n})`} />
      </td>
      <td style={{ ...td, whiteSpace: "nowrap" }}>
        {s.tokens.toLocaleString()} <span style={{ color: C_MUTED }}>· {(s.n ? s.tokens / s.n : 0).toFixed(0)} avg</span>
      </td>
    </>
  );
};

const Tile: React.FC<{ label: string; value: React.ReactNode; sub?: string }> = ({ label, value, sub }) => (
  <div style={{ border: `1px solid ${C_BORDER}`, borderRadius: 8, padding: "8px 12px", minWidth: 120 }}>
    <div style={{ fontSize: 11, color: C_MUTED }}>{label}</div>
    <div style={{ fontSize: 17, fontWeight: 700, color: C_TEXT }}>{value}</div>
    {sub && <div style={{ fontSize: 11, color: C_MUTED }}>{sub}</div>}
  </div>
);

export const ReliabilityDashboard: React.FC<ReliabilityDashboardProps> = ({ projectId }) => {
  const [allRows, setAllRows] = useState<ZeroShotCallRow[] | null>(null);
  const [capped, setCapped] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [reload, setReload] = useState(0);
  const [onlyThisProject, setOnlyThisProject] = useState(false);
  const [sortKey, setSortKey] = useState<SortKey>("n");
  const [sortDesc, setSortDesc] = useState(true);
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());

  useEffect(() => {
    let cancelled = false;
    setAllRows(null);
    setError(null);
    fetchAll()
      .then((r) => {
        if (cancelled) return;
        setAllRows(r.rows);
        setCapped(r.capped);
      })
      .catch((e) => {
        if (!cancelled) setError(e instanceof Error ? e.message : String(e));
      });
    return () => {
      cancelled = true;
    };
  }, [reload]);

  const filterActive = onlyThisProject && projectId !== null;
  const rows = useMemo(
    () => (allRows ? (filterActive ? allRows.filter((r) => r.project_id === projectId) : allRows) : []),
    [allRows, filterActive, projectId],
  );
  const nullProjectRows = useMemo(() => (allRows ? allRows.filter((r) => r.project_id === null).length : 0), [allRows]);
  const { total, groups } = useMemo(() => aggregateCalls(rows), [rows]);

  const sortedGroups = useMemo(() => {
    const dir = sortDesc ? -1 : 1;
    const cmp = (a: SiteGroup, b: SiteGroup): number =>
      sortKey === "call_site"
        ? dir * a.callSite.localeCompare(b.callSite)
        : dir * (metric(a.total, sortKey) - metric(b.total, sortKey)) || a.callSite.localeCompare(b.callSite);
    return groups
      .map((g) => ({
        ...g,
        models: [...g.models].sort((a, b) =>
          sortKey === "call_site"
            ? dir * modelLabel(a.model).localeCompare(modelLabel(b.model))
            : dir * (metric(a.stats, sortKey) - metric(b.stats, sortKey)) || modelLabel(a.model).localeCompare(modelLabel(b.model)),
        ),
      }))
      .sort(cmp);
  }, [groups, sortKey, sortDesc]);

  const seenSites = useMemo(() => new Set(groups.map((g) => g.callSite)), [groups]);
  const neverCaptured = KNOWN_CALL_SITES.filter((c) => !seenSites.has(c.name));
  const unknownSeen = groups.filter((g) => !KNOWN_NAMES.has(g.callSite)).map((g) => g.callSite);
  const modelCount = useMemo(() => new Set(rows.map((r) => r.model_used ?? "")).size, [rows]);
  const span = useMemo(() => {
    if (rows.length === 0) return null;
    let lo = rows[0].ts;
    let hi = rows[0].ts;
    for (const r of rows) {
      if (r.ts < lo) lo = r.ts;
      if (r.ts > hi) hi = r.ts;
    }
    return { lo, hi };
  }, [rows]);

  function toggleSort(key: SortKey) {
    if (key === sortKey) setSortDesc((d) => !d);
    else {
      setSortKey(key);
      setSortDesc(key !== "call_site");
    }
  }
  const th = (key: SortKey, label: string): React.ReactNode => (
    <th
      onClick={() => toggleSort(key)}
      aria-sort={sortKey === key ? (sortDesc ? "descending" : "ascending") : "none"}
      style={{ textAlign: "left", padding: "6px 10px", color: sortKey === key ? C_TEXT : C_MUTED, cursor: "pointer", whiteSpace: "nowrap", fontWeight: 600, borderBottom: `1px solid ${C_BORDER}`, userSelect: "none" }}
    >
      {label}
      {sortKey === key ? (sortDesc ? " ▾" : " ▴") : ""}
    </th>
  );
  const toggleGroup = (name: string) =>
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });

  return (
    <div style={{ fontSize: 12.5, color: C_BODY }}>
      <div style={{ display: "flex", alignItems: "center", gap: 12, flexWrap: "wrap", marginBottom: 8 }}>
        <span style={{ fontWeight: 700, color: C_TEXT }}>Per-call-site reliability</span>
        <button
          onClick={() => setReload((n) => n + 1)}
          style={{ background: "transparent", color: C_BODY, border: `1px solid ${C_BORDER}`, borderRadius: 6, padding: "2px 10px", cursor: "pointer", fontSize: 12 }}
        >
          Refresh
        </button>
        <label
          title={projectId === null ? "Select a project to filter" : `Only rows whose project_id is ${projectId}`}
          style={{ display: "flex", alignItems: "center", gap: 5, color: projectId === null ? C_MUTED : C_BODY, cursor: projectId === null ? "not-allowed" : "pointer" }}
        >
          <input
            type="checkbox"
            disabled={projectId === null}
            checked={filterActive}
            onChange={(e) => setOnlyThisProject(e.target.checked)}
            style={{ margin: 0 }}
          />
          This project only{projectId !== null ? ` (#${projectId})` : ""}
        </label>
      </div>
      <p style={{ margin: "0 0 10px", color: C_MUTED, lineHeight: 1.5 }}>
        Measured, not guessed — Methodology 38 (tune from measured metrics, never preemptive caps). Computed from every row of the
        zero-shot capture store (S11); it refreshes on demand only, because the store is a plain file append and emits no live event.
      </p>

      {error && (
        <div style={{ color: "#ff8a8a", marginBottom: 10 }}>
          Could not read the capture store: {error}
        </div>
      )}
      {!error && allRows === null && <div style={{ color: C_MUTED }}>Loading the capture store…</div>}

      {allRows !== null && (
        <>
          <div style={{ display: "flex", gap: 10, flexWrap: "wrap", marginBottom: 10 }}>
            <Tile label="Rows analysed" value={total.n.toLocaleString()} sub={capped ? `capped at ${ROW_CAP.toLocaleString()} — more rows exist (store order, oldest first)` : filterActive ? `of ${allRows.length.toLocaleString()} total` : "all rows in the store"} />
            <Tile label="Success" value={total.n ? `${pct(total.ok / total.n)}` : "—"} sub={`${total.ok}/${total.n} calls`} />
            <Tile label="Used fallback" value={total.n ? `${pct(total.fallback / total.n)}` : "—"} sub={`${total.fallback}/${total.n} calls`} />
            <Tile label="Call sites / models" value={`${groups.length} / ${modelCount}`} sub="seen in these rows" />
            {span && <Tile label="Span" value={span.lo.slice(0, 10) === span.hi.slice(0, 10) ? span.lo.slice(0, 10) : `${span.lo.slice(0, 10)} → ${span.hi.slice(0, 10)}`} sub="row timestamps" />}
          </div>
          {capped && (
            <div style={{ color: C_WARN, marginBottom: 8 }}>
              {total.n.toLocaleString()} rows analysed (capped) — the store holds more; these figures cover only the first {ROW_CAP.toLocaleString()} in store order.
            </div>
          )}
          {filterActive && nullProjectRows > 0 && (
            <div style={{ color: C_MUTED, marginBottom: 8 }}>
              {nullProjectRows} row{nullProjectRows === 1 ? "" : "s"} have no project_id and are excluded by the project filter.
            </div>
          )}
          {total.n > 0 && total.ok === 0 && (
            <div style={{ color: C_MUTED, marginBottom: 8 }}>
              No analysed call succeeded, so every success rate below is a measured 0% — not a rendering fault.
            </div>
          )}

          {total.n === 0 ? (
            <div style={{ color: C_MUTED }}>
              {allRows.length === 0
                ? "The capture store has no rows yet."
                : "No rows match this project — nothing to aggregate."}
            </div>
          ) : (
            <div style={{ overflowX: "auto" }}>
              <table style={{ borderCollapse: "collapse", width: "100%", fontSize: 12 }}>
                <thead>
                  <tr>
                    {th("call_site", "Call site / model")}
                    {th("n", "Calls (n)")}
                    {th("success", "Success rate")}
                    {th("retries", "Retries")}
                    {th("fallback", "Fallback rate")}
                    {th("tokens", "Tokens (total · mean)")}
                  </tr>
                </thead>
                <tbody>
                  {sortedGroups.map((g) => {
                    const isCollapsed = collapsed.has(g.callSite);
                    return (
                      <React.Fragment key={g.callSite}>
                        <tr style={{ borderTop: `1px solid ${C_BORDER}`, background: "rgba(255,255,255,0.02)" }}>
                          <td style={{ padding: "5px 10px", fontWeight: 600, color: C_TEXT }}>
                            <button
                              onClick={() => toggleGroup(g.callSite)}
                              aria-expanded={!isCollapsed}
                              style={{ background: "none", border: "none", color: C_MUTED, cursor: "pointer", padding: 0, marginRight: 6 }}
                            >
                              {isCollapsed ? "▸" : "▾"}
                            </button>
                            {g.callSite}
                            <span style={{ color: C_MUTED, fontWeight: 400 }}> · {g.models.length} model{g.models.length === 1 ? "" : "s"}</span>
                            {!KNOWN_NAMES.has(g.callSite) && (
                              <span title="Not in this view's known call-site list (static snapshot of the source) — the list may have drifted." style={{ marginLeft: 6, fontSize: 10.5, color: C_MUTED }}>
                                (not in known list)
                              </span>
                            )}
                          </td>
                          <StatCells s={g.total} strong />
                        </tr>
                        {!isCollapsed &&
                          g.models.map((m) => (
                            <tr key={`${g.callSite}|${m.model}`}>
                              <td style={{ padding: "4px 10px 4px 34px", color: m.model === "" ? C_MUTED : C_BODY }} title={m.model === "" ? 'model_used is "" — no model returned an answer for these calls' : undefined}>
                                {modelLabel(m.model)}
                              </td>
                              <StatCells s={m.stats} />
                            </tr>
                          ))}
                      </React.Fragment>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}

          {neverCaptured.length > 0 && (
            <div style={{ marginTop: 14 }}>
              <div style={{ color: C_TEXT, fontWeight: 600, marginBottom: 4 }}>
                Known call sites with no captured calls{filterActive ? " in this project" : ""} ({neverCaptured.length})
              </div>
              <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
                {neverCaptured.map((c) => (
                  <span
                    key={c.name}
                    title={`Passed as call_site to metered_execute_resilient at src/orchestrator/${c.source}`}
                    style={{ border: `1px dashed ${C_BORDER}`, borderRadius: 6, padding: "1px 8px", color: C_MUTED }}
                  >
                    {c.name} <span style={{ opacity: 0.7 }}>· {c.source}</span>
                  </span>
                ))}
              </div>
            </div>
          )}
          {unknownSeen.length > 0 && (
            <div style={{ marginTop: 10, color: C_MUTED }}>
              Seen in the store but missing from the known-site list: {unknownSeen.join(", ")}.
            </div>
          )}
          <div style={{ marginTop: 12, color: C_MUTED, lineHeight: 1.5 }}>
            Coverage: the store is written only by <code>metered_execute_resilient</code>, so it does not include the main
            step-execution dispatch, <code>confirm_yes_no_orch</code>, AMT re-expansion, or the methodology meta-loop
            (docs/ZERO_SHOT_CALL_REGISTRY.md §1). A site listed as having no captured calls may simply not have run yet.
          </div>
        </>
      )}
    </div>
  );
};

export default ReliabilityDashboard;
