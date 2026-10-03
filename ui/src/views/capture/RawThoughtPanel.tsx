/**
 * E1 — Raw Thought panel: every real model call the orchestrator made through
 * the zero-shot executor, exactly as captured (S11 `zero_shot_calls.jsonl`).
 *
 * Real sources (all confirmed, not assumed):
 *  - Route: GET /capture/zero-shot-calls (B5, src/grpc/mod.rs get_zero_shot_calls). Filters
 *    (project_id/call_site/model_used/amt_container_id/blueprint_id) apply BEFORE offset/limit,
 *    and the response carries `total` = the FILTERED row count (data/captureData.ts's
 *    CapturePage type does not declare it, so it is read here through a local widening type).
 *    File order is chronological (oldest first — checked on live data), so "newest first" is done
 *    by asking for the tail of the filtered set (offset = total - PAGE) and paging backwards.
 *  - Row semantics (src/orchestrator/mod.rs capture_zero_shot_call ~2560-2640):
 *      · `used_fallback` = the primary model result was unusable (error / empty / gate-rejected) so
 *        the fallback chain was tried; `model_used`/`success`/`response_preview` describe the FINAL
 *        outcome after that. `retry_count` = retries before falling back.
 *      · `model_used === ""` = no model produced a response at all (the error branch records "").
 *      · `success === false` with a non-empty `model_used` = a model answered but the result was
 *        judged unusable (is_unusable_pipeline9_result) — i.e. rejected, not absent.
 *      · `prompt_preview` is cut to 200 chars, `response_preview` to 500 (300 for error text) by the
 *        capture itself — the stored preview is all that exists, so nothing is truncated further here.
 *  - Design premise: every row on disk today is a FAILURE (OpenRouter credits exhausted) — failure
 *    is the primary state. Failure is signalled from the raw `success` field (not from E4's
 *    classifier, which is additive), rows are never hidden behind a "success" filter, and the summary
 *    strip leads with the failed/no-model counts.
 *
 * Read-only: nothing here writes to the host.
 */
import React, { useCallback, useEffect, useRef, useState } from "react";
import { fetchZeroShotCalls, ZeroShotCallRow, ZeroShotFilters } from "../../data/captureData";
import { getJson } from "../../data/http";
import { CaptureStatusBadge, classifyZeroShotCall } from "./captureStatus";

export type RawThoughtPanelProps = { projectId: number | null };

const PAGE = 50;
const FALLBACK_PAGE = 200;
const FALLBACK_MAX_PAGES = 10;
/** Sentinel for the "no model answered" filter (real rows with model_used === ""). */
const NO_MODEL = "__none__";

const C_TEXT = "var(--color-text)";
const C_BODY = "var(--color-text-secondary)";
const C_MUTED = "var(--color-text-muted)";
const C_BORDER = "var(--color-border-faint)";
const C_PANEL = "var(--color-bg)";
const C_FIELD = "#101724";
const C_FAIL = "#ff8a8a";
const C_OK = "#8fe38f";
const C_WARN = "#e8c14f";

type Scope = "all" | "project";
interface Filters {
  scope: Scope;
  callSite: string;
  /** "" = any model, NO_MODEL = model_used "", otherwise an exact model name. */
  model: string;
}

/** CapturePage plus the `total` the route really returns. */
interface PageResponse {
  limit: number;
  offset: number;
  rows: ZeroShotCallRow[];
  total?: number;
}

interface LoadedRow {
  row: ZeroShotCallRow;
  /** Position within the FILTERED set in file order — stable for a given filter set, used as the React key. */
  index: number;
}

async function fetchPage(projectId: number | null, f: Filters, offset: number, limit: number): Promise<PageResponse> {
  const project_id = f.scope === "project" && projectId !== null ? projectId : undefined;
  if (f.model === NO_MODEL) {
    // data/http.ts qs() drops empty values, so the model_used="" filter (rows where no model answered)
    // has to be built by hand. The route accepts it: verified live, it matches model_used == "".
    const parts = [`offset=${offset}`, `limit=${limit}`, "model_used="];
    if (project_id !== undefined) parts.push(`project_id=${project_id}`);
    if (f.callSite) parts.push(`call_site=${encodeURIComponent(f.callSite)}`);
    return getJson<PageResponse>(`/capture/zero-shot-calls?${parts.join("&")}`);
  }
  const opts: ZeroShotFilters = {
    offset,
    limit,
    project_id,
    call_site: f.callSite || undefined,
    model_used: f.model || undefined,
  };
  return fetchZeroShotCalls(opts);
}

function byTsDesc(a: LoadedRow, b: LoadedRow): number {
  return a.row.ts < b.row.ts ? 1 : a.row.ts > b.row.ts ? -1 : b.index - a.index;
}

function fmtTs(ts: string): string {
  const d = new Date(ts);
  return Number.isNaN(d.getTime()) ? ts : d.toLocaleString();
}

const chip: React.CSSProperties = {
  display: "inline-block",
  border: `1px solid ${C_BORDER}`,
  borderRadius: 999,
  padding: "0 7px",
  fontSize: 11,
  lineHeight: "18px",
  color: C_BODY,
  whiteSpace: "nowrap",
};

const selectStyle: React.CSSProperties = {
  background: C_FIELD,
  color: C_TEXT,
  border: `1px solid ${C_BORDER}`,
  borderRadius: 6,
  padding: "3px 8px",
  fontSize: 12,
};

const buttonStyle: React.CSSProperties = {
  background: "transparent",
  color: C_BODY,
  border: `1px solid ${C_BORDER}`,
  borderRadius: 6,
  padding: "3px 10px",
  fontSize: 12,
  cursor: "pointer",
};

const preStyle: React.CSSProperties = {
  margin: "2px 0 0",
  padding: "6px 8px",
  background: C_PANEL,
  border: `1px solid ${C_BORDER}`,
  borderRadius: 6,
  color: C_BODY,
  fontSize: 11.5,
  lineHeight: 1.5,
  whiteSpace: "pre-wrap",
  wordBreak: "break-word",
  overflowWrap: "anywhere",
  maxHeight: 240,
  overflowY: "auto",
};

const KV: React.FC<{ k: string; title?: string; children: React.ReactNode }> = ({ k, title, children }) => (
  <div style={{ display: "flex", gap: 6, minWidth: 0 }} title={title}>
    <span style={{ color: C_MUTED, flexShrink: 0 }}>{k}:</span>
    <span style={{ color: C_BODY, overflowWrap: "anywhere", minWidth: 0 }}>{children}</span>
  </div>
);

function idOrNone(v: number | null): React.ReactNode {
  return v === null ? (
    <span style={{ color: C_MUTED, fontStyle: "italic" }}>none recorded</span>
  ) : (
    v
  );
}

const CallRow: React.FC<{ item: LoadedRow; open: boolean; onToggle: () => void }> = ({ item, open, onToggle }) => {
  const r = item.row;
  const failed = !r.success;
  const noModel = r.model_used === "";
  const accent = failed ? C_FAIL : C_OK;
  const status = classifyZeroShotCall(r);
  return (
    <div
      style={{
        border: `1px solid ${C_BORDER}`,
        borderLeft: `3px solid ${accent}`,
        borderRadius: 8,
        background: failed ? "rgba(255,138,138,0.05)" : "transparent",
        marginBottom: 6,
        minWidth: 0,
      }}
    >
      <button
        onClick={onToggle}
        aria-expanded={open}
        style={{
          all: "unset",
          boxSizing: "border-box",
          width: "100%",
          cursor: "pointer",
          padding: "7px 10px",
          display: "flex",
          alignItems: "center",
          gap: 8,
          flexWrap: "wrap",
          fontSize: 12,
          color: C_BODY,
        }}
      >
        <span style={{ color: C_MUTED, width: 12 }}>{open ? "▾" : "▸"}</span>
        <CaptureStatusBadge status={status} />
        <span
          style={{ ...chip, color: failed ? C_FAIL : C_OK, borderColor: failed ? C_FAIL : C_OK }}
          title={
            failed
              ? noModel
                ? "success:false and no model answered — the primary attempt and any fallback both produced nothing usable."
                : "success:false but a model did answer — the result was judged unusable (rejected), see the response."
              : "success:true — a usable response was returned."
          }
        >
          {failed ? "failed" : "ok"}
        </span>
        <span style={{ color: C_MUTED }} title={r.ts}>
          {fmtTs(r.ts)}
        </span>
        <span style={{ ...chip, fontFamily: "monospace" }}>{r.call_site}</span>
        {noModel ? (
          <span style={{ ...chip, color: C_FAIL, borderColor: C_FAIL, fontStyle: "italic" }} title='model_used is "" — no model produced a response for this call.'>
            no model answered
          </span>
        ) : (
          <span style={{ ...chip, fontFamily: "monospace" }}>{r.model_used}</span>
        )}
        <span style={{ color: C_MUTED }}>{r.tokens_used} tok</span>
        <span style={{ color: r.retry_count > 0 ? C_WARN : C_MUTED }} title="Retries before falling back">
          {r.retry_count} {r.retry_count === 1 ? "retry" : "retries"}
        </span>
        {r.used_fallback && (
          <span
            style={{ ...chip, color: C_WARN, borderColor: C_WARN }}
            title="The primary result was unusable, so the fallback chain was tried. model_used / success below describe the final outcome after that."
          >
            fallback used
          </span>
        )}
      </button>

      {open && (
        <div style={{ padding: "4px 12px 10px 33px", fontSize: 12, color: C_BODY, display: "flex", flexDirection: "column", gap: 8 }}>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(210px, 1fr))", gap: "2px 16px" }}>
            <KV k="ts">{r.ts}</KV>
            <KV k="call_site">{r.call_site}</KV>
            <KV k="model_used">{noModel ? <i style={{ color: C_FAIL }}>"" (no model answered)</i> : r.model_used}</KV>
            <KV k="tokens_used">{r.tokens_used}</KV>
            <KV k="retry_count">{r.retry_count}</KV>
            <KV k="used_fallback">{String(r.used_fallback)}</KV>
            <KV k="success">{String(r.success)}</KV>
            <KV k="project_id" title="The project this orchestration request belonged to.">{idOrNone(r.project_id)}</KV>
            <KV k="amt_container_id" title="The AMT container persisted for this request. Null when the call happened before an AMT existed (e.g. intent extraction) or the AMT never persisted.">
              {idOrNone(r.amt_container_id)}
            </KV>
            <KV k="blueprint_id" title="The blueprint assigned to this request. Null when the call happened before blueprint assignment or it never completed.">
              {idOrNone(r.blueprint_id)}
            </KV>
          </div>
          <div>
            <div style={{ color: C_MUTED, fontSize: 11 }}>
              prompt_preview — as stored (the capture keeps only the first 200 characters of the prompt)
            </div>
            <pre style={preStyle}>{r.prompt_preview || "(empty)"}</pre>
          </div>
          <div>
            <div style={{ color: failed ? C_FAIL : C_MUTED, fontSize: 11 }}>
              response_preview — as stored (first 500 characters of a response, 300 of an error){failed ? " — this call failed" : ""}
            </div>
            <pre style={{ ...preStyle, borderColor: failed ? C_FAIL : C_BORDER }}>{r.response_preview || "(empty)"}</pre>
          </div>
        </div>
      )}
    </div>
  );
};

export const RawThoughtPanel: React.FC<RawThoughtPanelProps> = ({ projectId }) => {
  const [filters, setFilters] = useState<Filters>({ scope: "all", callSite: "", model: "" });
  const [rows, setRows] = useState<LoadedRow[]>([]);
  const [total, setTotal] = useState<number | null>(null);
  const [oldestOffset, setOldestOffset] = useState(0);
  const [totalUnreported, setTotalUnreported] = useState(false);
  const [phase, setPhase] = useState<"loading" | "ready" | "error">("loading");
  const [error, setError] = useState("");
  const [loadingOlder, setLoadingOlder] = useState(false);
  const [loadedAt, setLoadedAt] = useState<Date | null>(null);
  const [expanded, setExpanded] = useState<Set<number>>(new Set());
  const [refreshTick, setRefreshTick] = useState(0);
  // Grow-only option lists built from rows actually loaded, so narrowing a filter never removes the way back.
  const [knownSites, setKnownSites] = useState<Set<string>>(new Set());
  const [knownModels, setKnownModels] = useState<Set<string>>(new Set());
  const [sawNoModel, setSawNoModel] = useState(false);
  const reqRef = useRef(0);

  const noteOptions = useCallback((incoming: LoadedRow[]) => {
    if (incoming.length === 0) return;
    setKnownSites((prev) => {
      const next = new Set(prev);
      incoming.forEach((i) => next.add(i.row.call_site));
      return next;
    });
    setKnownModels((prev) => {
      const next = new Set(prev);
      incoming.forEach((i) => i.row.model_used !== "" && next.add(i.row.model_used));
      return next;
    });
    if (incoming.some((i) => i.row.model_used === "")) setSawNoModel(true);
  }, []);

  useEffect(() => {
    const token = ++reqRef.current;
    const stale = () => token !== reqRef.current;
    setPhase("loading");
    setError("");
    setExpanded(new Set());
    (async () => {
      try {
        const head = await fetchPage(projectId, filters, 0, 1);
        if (stale()) return;
        let loaded: LoadedRow[];
        if (typeof head.total === "number") {
          const t = head.total;
          const start = Math.max(0, t - PAGE);
          const page = t === 0 ? { rows: [] as ZeroShotCallRow[] } : await fetchPage(projectId, filters, start, t - start);
          if (stale()) return;
          loaded = page.rows.map((row, j) => ({ row, index: start + j })).sort(byTsDesc);
          setTotal(t);
          setOldestOffset(start);
          setTotalUnreported(false);
        } else {
          // Host predates the `total` field: page forward from the start (bounded), then order newest-first.
          const all: LoadedRow[] = [];
          let off = 0;
          for (let i = 0; i < FALLBACK_MAX_PAGES; i++) {
            const p = await fetchPage(projectId, filters, off, FALLBACK_PAGE);
            if (stale()) return;
            p.rows.forEach((row, j) => all.push({ row, index: off + j }));
            if (p.rows.length < FALLBACK_PAGE) break;
            off += FALLBACK_PAGE;
          }
          loaded = all.sort(byTsDesc);
          setTotal(null);
          setOldestOffset(0);
          setTotalUnreported(true);
        }
        setRows(loaded);
        noteOptions(loaded);
        setLoadedAt(new Date());
        setPhase("ready");
      } catch (e) {
        if (stale()) return;
        setError(e instanceof Error ? e.message : String(e));
        setPhase("error");
      }
    })();
    return () => {
      reqRef.current++;
    };
  }, [projectId, filters, refreshTick, noteOptions]);

  const loadOlder = async () => {
    if (oldestOffset <= 0 || loadingOlder) return;
    const token = reqRef.current;
    setLoadingOlder(true);
    try {
      const start = Math.max(0, oldestOffset - PAGE);
      const page = await fetchPage(projectId, filters, start, oldestOffset - start);
      if (token !== reqRef.current) return;
      const older = page.rows.map((row, j) => ({ row, index: start + j })).sort(byTsDesc);
      setRows((prev) => [...prev, ...older]);
      setOldestOffset(start);
      noteOptions(older);
    } catch (e) {
      if (token === reqRef.current) setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoadingOlder(false);
    }
  };

  const toggle = (index: number) =>
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(index)) next.delete(index);
      else next.add(index);
      return next;
    });

  const failedCount = rows.filter((i) => !i.row.success).length;
  const noModelCount = rows.filter((i) => i.row.model_used === "").length;
  const fallbackCount = rows.filter((i) => i.row.used_fallback).length;
  const filtered = filters.callSite !== "" || filters.model !== "" || filters.scope === "project";
  const hasOlder = oldestOffset > 0;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 10, minWidth: 0 }}>
      <div style={{ display: "flex", gap: 10, alignItems: "center", flexWrap: "wrap", fontSize: 12 }}>
        <span style={{ display: "inline-flex", border: `1px solid ${C_BORDER}`, borderRadius: 6, overflow: "hidden" }}>
          {(["all", "project"] as const).map((s) => (
            <button
              key={s}
              disabled={s === "project" && projectId === null}
              onClick={() => setFilters((f) => ({ ...f, scope: s }))}
              title={s === "project" && projectId === null ? "Select a project first" : undefined}
              style={{
                ...buttonStyle,
                border: "none",
                borderRadius: 0,
                background: filters.scope === s ? C_BORDER : "transparent",
                color: filters.scope === s ? C_TEXT : C_MUTED,
                opacity: s === "project" && projectId === null ? 0.5 : 1,
              }}
            >
              {s === "all" ? "All projects" : projectId !== null ? `Project ${projectId}` : "This project"}
            </button>
          ))}
        </span>
        <label style={{ color: C_MUTED, display: "inline-flex", gap: 6, alignItems: "center" }}>
          call site
          <select style={selectStyle} value={filters.callSite} onChange={(e) => setFilters((f) => ({ ...f, callSite: e.target.value }))}>
            <option value="">any</option>
            {Array.from(knownSites)
              .sort()
              .map((s) => (
                <option key={s} value={s}>
                  {s}
                </option>
              ))}
          </select>
        </label>
        <label style={{ color: C_MUTED, display: "inline-flex", gap: 6, alignItems: "center" }}>
          model
          <select style={selectStyle} value={filters.model} onChange={(e) => setFilters((f) => ({ ...f, model: e.target.value }))}>
            <option value="">any</option>
            {(sawNoModel || filters.model === NO_MODEL) && <option value={NO_MODEL}>no model answered</option>}
            {Array.from(knownModels)
              .sort()
              .map((m) => (
                <option key={m} value={m}>
                  {m}
                </option>
              ))}
          </select>
        </label>
        {filtered && (
          <button style={buttonStyle} onClick={() => setFilters({ scope: "all", callSite: "", model: "" })}>
            clear filters
          </button>
        )}
        <button style={{ ...buttonStyle, marginLeft: "auto" }} onClick={() => setRefreshTick((t) => t + 1)} disabled={phase === "loading"}>
          {phase === "loading" ? "loading…" : "refresh"}
        </button>
      </div>

      {phase === "error" && (
        <div style={{ border: `1px solid ${C_FAIL}`, color: C_FAIL, borderRadius: 8, padding: "8px 10px", fontSize: 12.5 }}>
          Could not read the capture store: {error}
          <button style={{ ...buttonStyle, marginLeft: 10 }} onClick={() => setRefreshTick((t) => t + 1)}>
            retry
          </button>
        </div>
      )}

      {phase === "loading" && rows.length === 0 && <div style={{ color: C_MUTED, fontSize: 12.5, padding: 8 }}>Loading model calls…</div>}

      {phase === "ready" && rows.length === 0 && (
        <div style={{ color: C_MUTED, fontSize: 12.5, padding: 12, border: `1px dashed ${C_BORDER}`, borderRadius: 8, lineHeight: 1.6 }}>
          {filtered
            ? filters.scope === "project"
              ? `No model calls are recorded for project ${projectId} with these filters. Calls with no project attribution, or belonging to other projects, appear under "All projects".`
              : "No captured model calls match these filters."
            : "No model calls captured yet — zero_shot_calls.jsonl is empty or absent. Nothing is shown in its place."}
        </div>
      )}

      {rows.length > 0 && (
        <>
          <div style={{ display: "flex", gap: 14, flexWrap: "wrap", fontSize: 12, color: C_MUTED, alignItems: "center" }}>
            <span style={{ color: C_TEXT }}>
              {total !== null ? `Showing newest ${rows.length} of ${total} matching call${total === 1 ? "" : "s"}` : `Loaded ${rows.length} call${rows.length === 1 ? "" : "s"}`}
            </span>
            <span style={{ color: failedCount > 0 ? C_FAIL : C_OK, fontWeight: 600 }} title="Counts cover the rows loaded so far, from each row's own success field.">
              {failedCount} of {rows.length} failed{failedCount === rows.length && rows.length > 0 ? " — none succeeded" : ""}
            </span>
            <span style={{ color: noModelCount > 0 ? C_FAIL : C_MUTED }}>{noModelCount} with no model answering</span>
            <span style={{ color: fallbackCount > 0 ? C_WARN : C_MUTED }}>{fallbackCount} used fallback</span>
            {loadedAt && <span>· loaded {loadedAt.toLocaleTimeString()}</span>}
          </div>
          {totalUnreported && (
            <div style={{ fontSize: 11.5, color: C_WARN }}>
              This host build did not report a total, so up to {FALLBACK_PAGE * FALLBACK_MAX_PAGES} rows were read from the start of the file; older/newer rows beyond that are not shown.
            </div>
          )}
          <div style={{ opacity: phase === "loading" ? 0.5 : 1, transition: "opacity 120ms" }} aria-busy={phase === "loading"}>
            {rows.map((item) => (
              <CallRow key={item.index} item={item} open={expanded.has(item.index)} onToggle={() => toggle(item.index)} />
            ))}
          </div>
          {hasOlder && (
            <button style={{ ...buttonStyle, alignSelf: "flex-start" }} onClick={loadOlder} disabled={loadingOlder}>
              {loadingOlder ? "loading…" : `load ${Math.min(PAGE, oldestOffset)} older`}
              {` (${oldestOffset} older not yet loaded)`}
            </button>
          )}
          {!hasOlder && !totalUnreported && <div style={{ fontSize: 11.5, color: C_MUTED }}>Start of the capture log reached.</div>}
        </>
      )}
    </div>
  );
};

export default RawThoughtPanel;
