/**
 * E5 — Model-switch timeline (Raw Thoughts panel, "Model switches" tab).
 *
 * Renders zero_shot_calls.jsonl (S11) as a chronological timeline of which
 * REAL model answered which call, with a "⇄ switched to X" divider wherever two
 * consecutive ANSWERED calls used different models (same visual language as the
 * chat transcript in components/MetaPortion.tsx).
 *
 * Real sources (all verified, not assumed):
 *  - Data: GET /capture/zero-shot-calls via data/captureData.ts (B5). Rows are
 *    appended to {general.data_dir}/model_calls/zero_shot_calls.jsonl by
 *    `capture_zero_shot_call` (src/orchestrator/mod.rs ~2592).
 *  - `model_used` is "" when the call returned Err or the result carried no
 *    model_used (mod.rs ~2608-2622) — i.e. NO MODEL ANSWERED. Those rows are
 *    real events and are shown as gaps, never skipped or merged. This is the
 *    primary state on today's data (every captured row is a failure — provider
 *    credits exhausted).
 *  - `used_fallback` = the primary model was still unusable after up to 2
 *    retries, so the fallback chain was walked (mod.rs ~2561-2580); the row
 *    records the FINAL outcome. `retry_count` counts retries against the primary.
 *  - `success:false` can coexist with a non-empty `model_used` (a model
 *    answered but the response was unusable) — the summary keeps
 *    "answered" and "succeeded" as separate facts.
 *  - `response_preview`/`prompt_preview` are truncated AT CAPTURE (errors 300
 *    chars, responses 500), so previews can end mid-token.
 *
 * Not offered (deliberately): "rerun this step under another model"
 * (POST /task/step/rerun exists, see TaskDetailPanel.doRerun). S11 rows carry
 * no task id or step index, so no row can honestly be tied to a rerunnable
 * step. Also no per-request grouping: S11 has no request id, so the only
 * grouping here is the optional project_id filter and plain time order.
 */
import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { fetchZeroShotCalls, ZeroShotCallRow, ZeroShotFilters } from "../../data/captureData";
import { CaptureStatusBadge, classifyZeroShotCall } from "./captureStatus";

export type ModelSwitchTimelineProps = { projectId: number | null };

// ── palette (matches components/GraphView.tsx) ──────────────────────────────
const C_TEXT = "#dfe7f2";
const C_BODY = "#c7d0dc";
const C_MUTED = "#8b98ab";
const C_BORDER = "#1e2836";
const C_PANEL = "#0a0f1a";
const C_WARN = "#e8c14f";
const C_ERR = "#ff8a8a";

// Same substring mapping as MetaPortion.tsx (not exported there; reproduced, not edited).
const MODEL_STYLES: Record<string, { color: string; icon: string }> = {
  bitnet: { color: "#22d3ee", icon: "⚡" },
  anthropic: { color: "#fb923c", icon: "◈" },
  claude: { color: "#fb923c", icon: "◈" },
  openrouter: { color: "#a78bfa", icon: "◇" },
  openai: { color: "#10b981", icon: "◆" },
  gpt: { color: "#10b981", icon: "◆" },
  zcode: { color: "#4ade80", icon: "❖" },
};
const DEFAULT_MODEL_STYLE = { color: "#6ec3ff", icon: "●" };
function modelStyle(model: string): { color: string; icon: string } {
  const key = Object.keys(MODEL_STYLES).find((k) => model.toLowerCase().includes(k));
  return key ? MODEL_STYLES[key] : DEFAULT_MODEL_STYLE;
}

// ── paging ──────────────────────────────────────────────────────────────────
const PAGE_SIZE = 200;
const LOAD_CAP = 1000; // rows per load (initial and per "Load more")
const WINDOW = 200; // events rendered at once

interface Loaded {
  rows: ZeroShotCallRow[];
  nextOffset: number;
  exhausted: boolean;
}

async function loadPages(filters: ZeroShotFilters, startOffset: number): Promise<Loaded> {
  const rows: ZeroShotCallRow[] = [];
  let offset = startOffset;
  while (rows.length < LOAD_CAP) {
    const page = await fetchZeroShotCalls({ ...filters, offset, limit: PAGE_SIZE });
    rows.push(...page.rows);
    offset += page.rows.length;
    // `page.limit` is the limit the host actually applied (it caps large requests).
    if (page.rows.length < Math.min(PAGE_SIZE, page.limit)) return { rows, nextOffset: offset, exhausted: true };
  }
  return { rows, nextOffset: offset, exhausted: false };
}

type LoadState =
  | { kind: "loading" }
  | { kind: "error"; message: string }
  | { kind: "ready"; data: Loaded; loadingMore: boolean; moreError?: string };

// ── derivation ──────────────────────────────────────────────────────────────
interface TimelineEvent {
  key: string;
  row: ZeroShotCallRow;
  ms: number | null;
  /** Elapsed since the previous call in this (filtered) sequence. */
  gapMs: number | null;
  /** Previously answering model when THIS row's model differs from it. */
  switchedFrom: string | null;
}

function parseMs(ts: string): number | null {
  const ms = Date.parse(ts);
  return Number.isNaN(ms) ? null : ms;
}

function fmtDuration(ms: number): string {
  if (ms < 1000) return `${Math.round(ms)}ms`;
  const s = ms / 1000;
  if (s < 60) return `${s.toFixed(1)}s`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m ${Math.round(s - m * 60)}s`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h}h ${m - h * 60}m`;
  const d = Math.floor(h / 24);
  return `${d}d ${h - d * 24}h`;
}

/** Chronological (ascending) event list. The file is append-only so host order
 * already is chronological; we still sort by parsed timestamp (stable, ties keep
 * file order) and fall back to file order if any timestamp fails to parse. */
function buildEvents(rows: ZeroShotCallRow[]): TimelineEvent[] {
  const indexed = rows.map((row, i) => ({ row, i, ms: parseMs(row.ts) }));
  if (indexed.every((x) => x.ms !== null)) {
    indexed.sort((a, b) => (a.ms as number) - (b.ms as number) || a.i - b.i);
  }
  const out: TimelineEvent[] = [];
  let lastAnswered: string | null = null;
  let prevMs: number | null = null;
  for (const { row, i, ms } of indexed) {
    let switchedFrom: string | null = null;
    if (row.model_used) {
      if (lastAnswered !== null && lastAnswered !== row.model_used) switchedFrom = lastAnswered;
      lastAnswered = row.model_used;
    }
    out.push({
      key: `r${i}`,
      row,
      ms,
      gapMs: ms !== null && prevMs !== null ? Math.max(0, ms - prevMs) : null,
      switchedFrom,
    });
    prevMs = ms;
  }
  return out;
}

interface ModelSummary {
  model: string; // "" = no model answered
  calls: number;
  succeeded: number;
  failed: number;
  fallback: number;
  tokens: number;
  retries: number;
}

function summarize(rows: ZeroShotCallRow[]): ModelSummary[] {
  const by = new Map<string, ModelSummary>();
  for (const r of rows) {
    let s = by.get(r.model_used);
    if (!s) {
      s = { model: r.model_used, calls: 0, succeeded: 0, failed: 0, fallback: 0, tokens: 0, retries: 0 };
      by.set(r.model_used, s);
    }
    s.calls += 1;
    if (r.success) s.succeeded += 1;
    else s.failed += 1;
    if (r.used_fallback) s.fallback += 1;
    s.tokens += r.tokens_used;
    s.retries += r.retry_count;
  }
  // Answering models first (most calls first), the "no model answered" bucket last.
  return Array.from(by.values()).sort((a, b) => {
    if (!a.model !== !b.model) return a.model ? -1 : 1;
    return b.calls - a.calls || a.model.localeCompare(b.model);
  });
}

// ── small presentational pieces ─────────────────────────────────────────────
const Pill: React.FC<{ children: React.ReactNode; color?: string; title?: string }> = ({ children, color = C_MUTED, title }) => (
  <span
    title={title}
    style={{
      border: `1px solid ${color}`,
      color,
      borderRadius: 999,
      fontSize: 11,
      padding: "1px 8px",
      whiteSpace: "nowrap",
    }}
  >
    {children}
  </span>
);

const Preview: React.FC<{ label: string; text: string; tone?: string }> = ({ label, text, tone = C_BODY }) => (
  <div style={{ marginTop: 8 }}>
    <div style={{ fontSize: 11, color: C_MUTED, marginBottom: 2 }}>{label}</div>
    <pre
      style={{
        margin: 0,
        whiteSpace: "pre-wrap",
        overflowWrap: "anywhere",
        maxHeight: 180,
        overflowY: "auto",
        background: C_PANEL,
        border: `1px solid ${C_BORDER}`,
        borderRadius: 6,
        padding: "6px 8px",
        fontSize: 11.5,
        lineHeight: 1.5,
        color: tone,
        fontFamily: "inherit",
      }}
    >
      {text === "" ? "(empty)" : text}
    </pre>
  </div>
);

const SummaryCard: React.FC<{ s: ModelSummary }> = ({ s }) => {
  const answered = s.model !== "";
  const ms = answered ? modelStyle(s.model) : null;
  const color = ms ? ms.color : C_ERR;
  return (
    <div
      style={{
        border: `1px ${answered ? "solid" : "dashed"} ${color}`,
        borderRadius: 8,
        padding: "8px 10px",
        minWidth: 150,
        maxWidth: 240,
        background: color + "0d",
      }}
    >
      <div style={{ fontSize: 12, fontWeight: 700, color, overflowWrap: "anywhere" }}>
        {ms ? `${ms.icon} ${s.model}` : "∅ no model answered"}
      </div>
      <div style={{ fontSize: 11.5, color: C_BODY, marginTop: 4, lineHeight: 1.55 }}>
        <div>
          {s.calls} call{s.calls === 1 ? "" : "s"}
          {answered ? ` · ${s.succeeded} ok · ${s.failed} unusable` : " · all failed"}
        </div>
        <div>
          {s.fallback} via fallback chain · {s.retries} retr{s.retries === 1 ? "y" : "ies"}
        </div>
        <div>{s.tokens} tokens</div>
      </div>
    </div>
  );
};

const SwitchDivider: React.FC<{ to: string; from: string }> = ({ to, from }) => {
  const ms = modelStyle(to);
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 10, margin: "8px 0 8px 22px" }}>
      <span style={{ flex: 1, height: 1, background: "#223046" }} />
      <span
        title={`Previous answering model: ${from}`}
        style={{
          border: `1px solid ${ms.color}`,
          color: ms.color,
          background: ms.color + "14",
          borderRadius: 999,
          fontSize: 11,
          padding: "3px 10px",
          whiteSpace: "nowrap",
        }}
      >
        {ms.icon} ⇄ switched to {to}
        <span style={{ color: C_MUTED }}> (from {from})</span>
      </span>
      <span style={{ flex: 1, height: 1, background: "#223046" }} />
    </div>
  );
};

const EventCard: React.FC<{ ev: TimelineEvent; open: boolean; onToggle: () => void }> = ({ ev, open, onToggle }) => {
  const r = ev.row;
  const answered = r.model_used !== "";
  const ms = answered ? modelStyle(r.model_used) : null;
  const dot = ms ? ms.color : C_ERR;
  const status = classifyZeroShotCall(r);
  return (
    <div style={{ display: "flex", gap: 10, alignItems: "stretch" }}>
      <div style={{ width: 12, display: "flex", flexDirection: "column", alignItems: "center" }}>
        <span
          style={{
            width: 10,
            height: 10,
            borderRadius: 999,
            marginTop: 8,
            boxSizing: "border-box",
            background: answered ? dot : "transparent",
            border: `2px ${answered ? "solid" : "dashed"} ${dot}`,
          }}
        />
        <span style={{ flex: 1, width: 1, background: "#223046" }} />
      </div>
      <div
        style={{
          flex: 1,
          minWidth: 0,
          border: `1px ${answered ? "solid" : "dashed"} ${answered ? C_BORDER : C_ERR + "88"}`,
          borderRadius: 8,
          padding: "8px 10px",
          marginBottom: 6,
          background: answered ? "transparent" : C_ERR + "08",
        }}
      >
        <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap", fontSize: 12 }}>
          <span title={r.ts} style={{ color: C_MUTED }}>
            {ev.ms !== null ? new Date(ev.ms).toLocaleString() : r.ts}
          </span>
          {ev.gapMs !== null && (
            <span style={{ color: C_MUTED, fontSize: 11 }} title="Time since the previous call in this view">
              +{fmtDuration(ev.gapMs)}
            </span>
          )}
          <Pill color={C_BODY}>{r.call_site}</Pill>
          {ms ? (
            <Pill color={ms.color}>
              {ms.icon} {r.model_used}
            </Pill>
          ) : (
            <Pill color={C_ERR} title="model_used is empty: the call errored or returned no model — nothing answered">
              ∅ no model answered
            </Pill>
          )}
          <CaptureStatusBadge status={status} />
          {r.used_fallback && (
            <Pill color={C_WARN} title="The primary model was unusable after retries, so the fallback chain was walked">
              fallback chain used
            </Pill>
          )}
          {r.retry_count > 0 && <Pill title="Retries against the primary model before falling back">{r.retry_count} retries</Pill>}
          {answered && !r.success && (
            <Pill color={C_ERR} title="A model answered but the response was judged unusable">
              answered, unusable
            </Pill>
          )}
          <span style={{ color: C_MUTED, fontSize: 11 }}>{r.tokens_used} tok</span>
          <button
            onClick={onToggle}
            style={{
              marginLeft: "auto",
              background: "none",
              border: `1px solid ${C_BORDER}`,
              color: C_MUTED,
              borderRadius: 999,
              fontSize: 11,
              padding: "1px 9px",
              cursor: "pointer",
            }}
          >
            {open ? "▾ hide" : "▸ details"}
          </button>
        </div>
        {open && (
          <div style={{ fontSize: 12, color: C_BODY }}>
            <div style={{ marginTop: 8, color: C_MUTED, fontSize: 11.5, lineHeight: 1.6, overflowWrap: "anywhere" }}>
              project: {r.project_id ?? "none"} · AMT container: {r.amt_container_id ?? "none"} · blueprint: {r.blueprint_id ?? "none"}
            </div>
            <Preview label="Prompt preview (truncated at capture)" text={r.prompt_preview} />
            <Preview
              label={answered ? "Response preview (truncated at capture, 500 chars)" : "Error returned — no model answered (truncated at capture, 300 chars)"}
              text={r.response_preview}
              tone={answered ? C_BODY : C_ERR}
            />
          </div>
        )}
      </div>
    </div>
  );
};

// ── component ───────────────────────────────────────────────────────────────
export const ModelSwitchTimeline: React.FC<ModelSwitchTimelineProps> = ({ projectId }) => {
  const [scopeToProject, setScopeToProject] = useState(true);
  const [callSite, setCallSite] = useState("");
  const [newestFirst, setNewestFirst] = useState(false);
  const [visible, setVisible] = useState(WINDOW);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [state, setState] = useState<LoadState>({ kind: "loading" });
  const [reloadTick, setReloadTick] = useState(0);
  const requestSeq = useRef(0);

  const effectiveProject = scopeToProject && projectId !== null ? projectId : null;
  const filters: ZeroShotFilters = useMemo(
    () => (effectiveProject !== null ? { project_id: effectiveProject } : {}),
    [effectiveProject],
  );

  useEffect(() => {
    const seq = ++requestSeq.current;
    setState({ kind: "loading" });
    setVisible(WINDOW);
    setExpanded(new Set());
    setCallSite("");
    loadPages(filters, 0)
      .then((data) => {
        if (seq === requestSeq.current) setState({ kind: "ready", data, loadingMore: false });
      })
      .catch((e) => {
        if (seq === requestSeq.current) setState({ kind: "error", message: e instanceof Error ? e.message : String(e) });
      });
  }, [filters, reloadTick]);

  const loadMore = useCallback(() => {
    if (state.kind !== "ready" || state.loadingMore || state.data.exhausted) return;
    const seq = requestSeq.current;
    const prev = state.data;
    setState({ kind: "ready", data: prev, loadingMore: true });
    loadPages(filters, prev.nextOffset)
      .then((more) => {
        if (seq !== requestSeq.current) return;
        setState({
          kind: "ready",
          loadingMore: false,
          data: { rows: prev.rows.concat(more.rows), nextOffset: more.nextOffset, exhausted: more.exhausted },
        });
      })
      .catch((e) => {
        if (seq !== requestSeq.current) return;
        setState({ kind: "ready", data: prev, loadingMore: false, moreError: e instanceof Error ? e.message : String(e) });
      });
  }, [state, filters]);

  const allRows = state.kind === "ready" ? state.data.rows : [];
  const callSites = useMemo(() => Array.from(new Set(allRows.map((r) => r.call_site))).sort(), [allRows]);
  const shownRows = useMemo(() => (callSite ? allRows.filter((r) => r.call_site === callSite) : allRows), [allRows, callSite]);
  const events = useMemo(() => buildEvents(shownRows), [shownRows]);
  const summaries = useMemo(() => summarize(shownRows), [shownRows]);

  const answeredCount = shownRows.filter((r) => r.model_used !== "").length;
  const noModelCount = shownRows.length - answeredCount;
  const distinctModels = summaries.filter((s) => s.model !== "").length;

  // Window anchored to the end the user reads first (latest rows when ascending).
  const ordered = newestFirst ? [...events].reverse() : events;
  const windowed = newestFirst ? ordered.slice(0, visible) : ordered.slice(Math.max(0, ordered.length - visible));
  const hiddenCount = ordered.length - windowed.length;

  const toggle = (key: string) =>
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });

  const btn: React.CSSProperties = {
    background: "#101724",
    color: C_BODY,
    border: `1px solid ${C_BORDER}`,
    borderRadius: 6,
    padding: "3px 10px",
    fontSize: 12,
    cursor: "pointer",
  };

  return (
    <div style={{ color: C_BODY }}>
      <div style={{ fontSize: 12.5, fontWeight: 700, color: C_TEXT }}>Model-switch timeline</div>
      <p style={{ fontSize: 11.5, color: C_MUTED, margin: "4px 0 10px", lineHeight: 1.5 }}>
        Every captured model call (zero_shot_calls.jsonl), in time order, showing which real model answered. Calls where no model answered are
        shown as gaps, not hidden. Captured calls carry no request or task id, so nothing is grouped per request — only optionally by project.
      </p>

      <div style={{ display: "flex", gap: 12, alignItems: "center", flexWrap: "wrap", marginBottom: 10, fontSize: 12 }}>
        <label style={{ display: "flex", alignItems: "center", gap: 5, color: projectId === null ? C_MUTED : C_BODY }}>
          <input
            type="checkbox"
            style={{ margin: 0 }}
            checked={effectiveProject !== null}
            disabled={projectId === null}
            onChange={(e) => setScopeToProject(e.target.checked)}
          />
          {projectId === null ? "All projects (no project selected)" : `Only project ${projectId}`}
        </label>
        <label style={{ display: "flex", alignItems: "center", gap: 5, color: C_MUTED }}>
          Call site:
          <select
            value={callSite}
            onChange={(e) => {
              setCallSite(e.target.value);
              setVisible(WINDOW);
            }}
            style={{ background: "#101724", color: C_TEXT, border: `1px solid ${C_BORDER}`, borderRadius: 6, padding: "3px 6px" }}
          >
            <option value="">all</option>
            {callSites.map((c) => (
              <option key={c} value={c}>
                {c}
              </option>
            ))}
          </select>
        </label>
        <button style={btn} onClick={() => setNewestFirst((v) => !v)}>
          {newestFirst ? "Newest first ↓" : "Oldest first ↓"}
        </button>
        <button style={btn} onClick={() => setReloadTick((t) => t + 1)}>
          ↻ Reload
        </button>
      </div>

      {state.kind === "loading" && <div style={{ padding: 16, color: C_MUTED }}>Loading captured calls…</div>}
      {state.kind === "error" && (
        <div style={{ padding: 16, color: C_ERR }}>Could not load /capture/zero-shot-calls: {state.message}</div>
      )}

      {state.kind === "ready" && allRows.length === 0 && (
        <div style={{ padding: 16, color: C_MUTED, fontSize: 12.5, lineHeight: 1.6 }}>
          {effectiveProject !== null ? (
            <>
              No model calls are recorded for project {effectiveProject}.{" "}
              <button style={btn} onClick={() => setScopeToProject(false)}>
                Show calls from all projects
              </button>
            </>
          ) : (
            "No model calls have been captured yet (zero_shot_calls.jsonl is empty or missing)."
          )}
        </div>
      )}

      {state.kind === "ready" && allRows.length > 0 && (
        <>
          <div style={{ fontSize: 12, color: C_BODY, marginBottom: 8, lineHeight: 1.6 }}>
            {shownRows.length} call{shownRows.length === 1 ? "" : "s"} loaded
            {callSite ? ` (call site ${callSite})` : ""}: <b>{answeredCount}</b> answered by a model
            {distinctModels > 0 ? ` (${distinctModels} distinct model${distinctModels === 1 ? "" : "s"})` : ""},{" "}
            <b style={{ color: noModelCount > 0 ? C_ERR : C_BODY }}>{noModelCount}</b> where no model answered.
            {distinctModels === 0 && (
              <span style={{ color: C_WARN }}> No model has answered any of these calls, so there is no model switch to show yet.</span>
            )}
            {distinctModels === 1 && <span style={{ color: C_MUTED }}> Only one model has answered — no switches so far.</span>}
          </div>

          <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginBottom: 12 }}>
            {summaries.map((s) => (
              <SummaryCard key={s.model || "__none__"} s={s} />
            ))}
          </div>

          {hiddenCount > 0 && !newestFirst && (
            <button style={{ ...btn, marginBottom: 8 }} onClick={() => setVisible((v) => v + WINDOW)}>
              ▲ Show {Math.min(WINDOW, hiddenCount)} earlier ({hiddenCount} not shown)
            </button>
          )}

          <div>
            {windowed.map((ev) => {
              // The switch divider sits BETWEEN the older and newer call it relates to,
              // so it goes before the card oldest-first and after it newest-first.
              const divider = ev.switchedFrom !== null ? <SwitchDivider to={ev.row.model_used} from={ev.switchedFrom} /> : null;
              return (
                <React.Fragment key={ev.key}>
                  {!newestFirst && divider}
                  <EventCard ev={ev} open={expanded.has(ev.key)} onToggle={() => toggle(ev.key)} />
                  {newestFirst && divider}
                </React.Fragment>
              );
            })}
          </div>

          {hiddenCount > 0 && newestFirst && (
            <button style={{ ...btn, marginTop: 4 }} onClick={() => setVisible((v) => v + WINDOW)}>
              ▼ Show {Math.min(WINDOW, hiddenCount)} more ({hiddenCount} not shown)
            </button>
          )}

          <div style={{ marginTop: 12, fontSize: 11.5, color: C_MUTED, lineHeight: 1.6 }}>
            {state.data.exhausted ? (
              <>All {allRows.length} matching captured calls are loaded.</>
            ) : (
              <>
                Loaded the first {allRows.length} captured calls (oldest first) — newer calls exist and are not shown yet.{" "}
                <button style={btn} disabled={state.loadingMore} onClick={loadMore}>
                  {state.loadingMore ? "Loading…" : `Load up to ${LOAD_CAP} more`}
                </button>
              </>
            )}
            {state.moreError && <span style={{ color: C_ERR }}> Could not load more: {state.moreError}</span>}
            <br />
            Rerunning a step under a different model isn't offered here: captured calls carry no task or step identifier, so no row can be tied to a
            rerunnable step. Use the Task viewer for reruns.
          </div>
        </>
      )}
    </div>
  );
};

export default ModelSwitchTimeline;
