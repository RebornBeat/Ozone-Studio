/**
 * J3 — Unified search bar (compact, mountable in a header/toolbar; no props).
 *
 * Real mechanism, verified live against the running host before building:
 * `SearchContainersByKeywords` (via `zseiQuery`, ../../ozoneClient) matches
 * container KEYWORDS with `strategy:"scan"` (substring, checked live: "math"
 * → 22 hits across every container type) or `"exact"`. It does NOT search
 * container `name`/content — real containers checked live have very uneven
 * keyword coverage: a real code-modality graph (id 1033) carries genuinely
 * useful keywords (`["__init__","add","calculator","python",...]`, real
 * function/class names), while a real text-modality graph (id 1060) and a
 * real AMT/Derived container (id 1061) both have `keywords: []` even though
 * their `name` field is a real, readable description ("Unknown text graph
 * (25 words...)", the AMT task prompt). CoordinationEvent containers always
 * carry real keywords (kind/agent/scope — confirmed live: 267 real events
 * under `ws:0` alone). This is stated plainly in the UI below rather than
 * silently — a search here can genuinely miss a real text/AMT container by
 * name because there is no full-text index, only a keyword one.
 *
 * Capture stores (S10/S11, `../../data/captureData`) are flat files, not ZSEI
 * containers — no keyword-search endpoint exists for them. "Search" over
 * them means client-side substring matching over a capped, already-fetched
 * page (500 rows each) — stated explicitly when a search is capped.
 */
import React, { useMemo, useRef, useState } from "react";
import { zseiQuery } from "../../ozoneClient";
import { fetchDecisionReviews, fetchZeroShotCalls, DecisionReviewRow, ZeroShotCallRow } from "../../data/captureData";
import { navigateTo } from "../../navigation";

type ResultGroup = "code" | "math" | "text" | "coordination" | "other-graph" | "decision-review" | "zero-shot-call";

interface ContainerResult {
  kind: "container";
  group: ResultGroup;
  containerId: number;
  title: string;
  subtitle?: string;
}
interface CaptureResult {
  kind: "decision-review" | "zero-shot-call";
  group: ResultGroup;
  title: string;
  subtitle: string;
}
type SearchResult = ContainerResult | CaptureResult;

const CAPTURE_SEARCH_CAP = 500;

function modalityFromObjectStorePath(path: unknown): "code" | "math" | "text" | null {
  if (typeof path !== "string") return null;
  if (path.startsWith("graphs/code_")) return "code";
  if (path.startsWith("graphs/math_")) return "math";
  if (path.startsWith("graphs/text_")) return "text";
  return null;
}

async function searchContainers(term: string): Promise<ContainerResult[]> {
  const res = await zseiQuery<any>({ SearchContainersByKeywords: { keywords: [term], strategy: "scan" } });
  const ids: number[] = res?.Containers ?? [];
  const capped = ids.slice(0, 60);
  const containers = await Promise.all(
    capped.map(async (id) => {
      const c = (await zseiQuery<any>({ GetContainer: { container_id: id } }))?.Container;
      return { id, c };
    }),
  );
  const out: ContainerResult[] = [];
  for (const { id, c } of containers) {
    if (!c) continue;
    const type: string = c.local_state?.metadata?.container_type ?? "Unknown";
    const name: string = c.local_state?.metadata?.name ?? `Container ${id}`;
    const keywords: string[] = c.local_state?.context?.keywords ?? [];
    let group: ResultGroup;
    if (type === "CoordinationEvent") group = "coordination";
    else if (type === "ModalityGraph") {
      group = modalityFromObjectStorePath(c.local_state?.storage?.object_store_path) ?? "other-graph";
    } else group = "other-graph";
    out.push({
      kind: "container",
      group,
      containerId: id,
      title: name,
      subtitle: `${type}${keywords.length ? ` · matched: ${keywords.filter((k) => k.toLowerCase().includes(term.toLowerCase())).join(", ")}` : ""}`,
    });
  }
  return out;
}

function matchesCapture(term: string, ...fields: (string | number | boolean | null | undefined)[]): boolean {
  const t = term.toLowerCase();
  return fields.some((f) => f !== null && f !== undefined && String(f).toLowerCase().includes(t));
}

async function searchZeroShotCalls(term: string): Promise<{ results: CaptureResult[]; capped: boolean }> {
  const page = await fetchZeroShotCalls({ limit: CAPTURE_SEARCH_CAP });
  const capped = page.rows.length >= CAPTURE_SEARCH_CAP;
  const hits: ZeroShotCallRow[] = page.rows.filter((r) =>
    matchesCapture(term, r.call_site, r.model_used, r.prompt_preview, r.response_preview),
  );
  return {
    capped,
    results: hits.slice(0, 30).map((r) => ({
      kind: "zero-shot-call",
      group: "zero-shot-call",
      title: `${r.call_site} · ${r.model_used || "no model answered"}`,
      subtitle: `${r.ts} · ${r.success ? "success" : "failure"} · ${r.prompt_preview.slice(0, 80)}`,
    })),
  };
}

async function searchDecisionReviews(term: string): Promise<{ results: CaptureResult[]; capped: boolean }> {
  const page = await fetchDecisionReviews({ limit: CAPTURE_SEARCH_CAP });
  const capped = page.rows.length >= CAPTURE_SEARCH_CAP;
  const hits: DecisionReviewRow[] = page.rows.filter((r) =>
    matchesCapture(term, r.model_used, r.decision, r.task_summary_preview, r.reasoning_preview),
  );
  return {
    capped,
    results: hits.slice(0, 30).map((r) => ({
      kind: "decision-review",
      group: "decision-review",
      title: `${r.decision} · ${r.model_used}`,
      subtitle: `${r.ts} · ${r.task_summary_preview.slice(0, 80)}`,
    })),
  };
}

const GROUP_LABEL: Record<ResultGroup, string> = {
  code: "Code graphs",
  math: "Math graphs",
  text: "Text graphs",
  "other-graph": "Other containers",
  coordination: "Coordination events",
  "decision-review": "Decision reviews",
  "zero-shot-call": "Model calls",
};
const GROUP_ORDER: ResultGroup[] = ["code", "math", "text", "coordination", "other-graph", "decision-review", "zero-shot-call"];

export const UnifiedSearch: React.FC = () => {
  const [open, setOpen] = useState(false);
  const [term, setTerm] = useState("");
  const [status, setStatus] = useState<"idle" | "loading" | "ready" | "error">("idle");
  const [error, setError] = useState<string | null>(null);
  const [results, setResults] = useState<SearchResult[]>([]);
  const [cappedNotes, setCappedNotes] = useState<string[]>([]);
  const requestId = useRef(0);

  const grouped = useMemo(() => {
    const byGroup = new Map<ResultGroup, SearchResult[]>();
    for (const r of results) {
      const list = byGroup.get(r.group) ?? [];
      list.push(r);
      byGroup.set(r.group, list);
    }
    return GROUP_ORDER.filter((g) => byGroup.has(g)).map((g) => ({ group: g, items: byGroup.get(g)! }));
  }, [results]);

  async function runSearch(q: string) {
    const trimmed = q.trim();
    if (trimmed.length < 2) {
      setStatus("idle");
      setResults([]);
      return;
    }
    const myRequest = ++requestId.current;
    setStatus("loading");
    setError(null);
    try {
      const [containerHits, callHits, reviewHits] = await Promise.all([
        searchContainers(trimmed).catch(() => [] as ContainerResult[]),
        searchZeroShotCalls(trimmed),
        searchDecisionReviews(trimmed),
      ]);
      if (myRequest !== requestId.current) return; // a newer search superseded this one
      const notes: string[] = [];
      if (callHits.capped) notes.push(`Model-call search only checked the most recent ${CAPTURE_SEARCH_CAP} rows.`);
      if (reviewHits.capped) notes.push(`Decision-review search only checked the most recent ${CAPTURE_SEARCH_CAP} rows.`);
      setCappedNotes(notes);
      setResults([...containerHits, ...callHits.results, ...reviewHits.results]);
      setStatus("ready");
    } catch (e) {
      if (myRequest !== requestId.current) return;
      setError(e instanceof Error ? e.message : String(e));
      setStatus("error");
    }
  }

  function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    runSearch(term);
  }

  function onResultClick(r: SearchResult) {
    if (r.kind === "container") navigateTo({ kind: "container", containerId: r.containerId });
    setOpen(false);
  }

  return (
    <div style={{ position: "relative", fontSize: 12.5 }}>
      <form onSubmit={onSubmit} style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <input
          value={term}
          onChange={(e) => setTerm(e.target.value)}
          onFocus={() => setOpen(true)}
          placeholder="Search containers, coordination events, model calls..."
          style={{
            background: "#101724",
            color: "var(--color-text)",
            border: "1px solid var(--color-border-faint)",
            borderRadius: 6,
            padding: "5px 10px",
            fontSize: 12.5,
            width: 280,
          }}
        />
        <button
          type="submit"
          style={{
            background: "var(--color-border-faint)",
            color: "var(--color-text)",
            border: "1px solid #2c3a4f",
            borderRadius: 6,
            padding: "5px 10px",
            fontSize: 12,
            cursor: "pointer",
          }}
        >
          Search
        </button>
      </form>

      {open && term.trim().length > 0 && (
        <div
          style={{
            position: "absolute",
            top: "calc(100% + 4px)",
            left: 0,
            width: 420,
            maxHeight: 420,
            overflowY: "auto",
            background: "var(--color-bg)",
            border: "1px solid var(--color-border-faint)",
            borderRadius: 8,
            padding: 10,
            zIndex: 50,
            boxShadow: "0 8px 24px rgba(0,0,0,0.4)",
          }}
        >
          <div style={{ fontSize: 10.5, color: "var(--color-text-muted)", marginBottom: 6, lineHeight: 1.5 }}>
            Matches container <b>keywords</b> (not names or full text) plus a capped scan of recent model calls and
            decision reviews. A container with no real keywords (common for text-graph and AMT containers) won't be
            found here even if its name matches — this is a real gap in what's searchable today, not a bug.
          </div>

          {status === "loading" && <div style={{ color: "var(--color-text-muted)" }}>Searching…</div>}
          {status === "error" && <div style={{ color: "#ff8a8a" }}>Error: {error}</div>}
          {status === "ready" && results.length === 0 && (
            <div style={{ color: "var(--color-text-muted)" }}>No matches for "{term.trim()}" in any searchable source.</div>
          )}
          {cappedNotes.map((n) => (
            <div key={n} style={{ color: "#e8c14f", fontSize: 10.5, marginBottom: 4 }}>
              {n}
            </div>
          ))}

          {grouped.map(({ group, items }) => (
            <div key={group} style={{ marginBottom: 10 }}>
              <div style={{ fontSize: 10.5, fontWeight: 700, textTransform: "uppercase", color: "var(--color-text-muted)", marginBottom: 4 }}>
                {GROUP_LABEL[group]} ({items.length})
              </div>
              {items.map((r, i) => (
                <div
                  key={i}
                  onClick={() => onResultClick(r)}
                  style={{
                    padding: "5px 6px",
                    borderRadius: 5,
                    cursor: r.kind === "container" ? "pointer" : "default",
                    color: "var(--color-text-secondary)",
                  }}
                  onMouseEnter={(e) => (e.currentTarget.style.background = "#101724")}
                  onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
                >
                  <div style={{ color: "var(--color-text)" }}>{r.title}</div>
                  {r.subtitle && <div style={{ fontSize: 11, color: "var(--color-text-muted)" }}>{r.subtitle}</div>}
                </div>
              ))}
            </div>
          ))}
        </div>
      )}
      {open && (
        <div
          onClick={() => setOpen(false)}
          style={{ position: "fixed", inset: 0, zIndex: 40 }}
          aria-hidden="true"
        />
      )}
    </div>
  );
};

export default UnifiedSearch;
