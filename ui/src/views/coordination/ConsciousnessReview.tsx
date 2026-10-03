/**
 * I5 — Manual consciousness-review-pass trigger (Coordination tab).
 *
 * POST /consciousness/review_pass (src/grpc/mod.rs:714, no request body)
 * runs `consciousness::review::run_review_pass` for real — it reads the real
 * decision-review capture store + recent tasks, traverses the graph, and
 * persists any genuine finding as a real `Derived` container under
 * `CONSCIOUSNESS_METACOGNITION_ROOT_ID` (id 54). Response:
 * `{ insight_container_ids: number[] }`. An empty array is a CORRECT, honest
 * outcome (nothing warranted flagging this pass), not an error — the plan
 * doc's own doctrine, and the route's own doc comment, say so explicitly.
 *
 * VERIFIED LIVE while building this (2026-09-27): triggered one real pass —
 * `{"insight_container_ids":[40194]}` — this system's first-ever real
 * consciousness insight. Its container (`persist_insight`,
 * src/consciousness/review.rs:262-336) is real and reachable via
 * `GetContainer`: `container_type:"Derived"`, `parent_id:54`,
 * `name:"Consciousness insight: {kind}"`,
 * `materialized_path:"/Consciousness/Metacognition/{kind}"`,
 * `keywords:[kind,"review-pass"]`, real `created_at`.
 *
 * REAL GAP FOUND (not fixable from this file — flagged in the handoff, not
 * worked around): `persist_insight` writes the actual `content`/`citations`
 * text by injecting them as EXTRA keys onto the serialized container's raw
 * JSON (`value["content"] = ...; value["citations"] = ...`) — fields that
 * exist in the on-disk document but are OUTSIDE the typed `Container`
 * struct's schema. `GetContainer` deserializes into that typed struct, so
 * serde silently drops them. `GetContainerContent` only reads a file at
 * `object_store_path`, which `persist_insight` never sets (confirmed `null`
 * on the real container above) — there is no `object_store_path` file to
 * read. Net result: the real insight text is genuinely persisted but
 * UNREACHABLE through any current `ZSEIQuery` variant. This view is honest
 * about that rather than showing a fake blank "content" field.
 */
import React, { useState } from "react";
import { zseiQuery } from "../../ozoneClient";

export type ConsciousnessReviewProps = { projectId: number | null };

interface RawContainer {
  global_state: { container_id: number; parent_id: number };
  local_state: {
    metadata: {
      container_type: string;
      name?: string | null;
      materialized_path?: string | null;
      provenance: string;
      created_at: number;
    };
    context: { keywords: string[] };
    storage: { object_store_path?: string | null };
  };
}

interface ResolvedInsight {
  containerId: number;
  kind: string;
  name: string;
  materializedPath: string;
  provenance: string;
  createdAt: number;
  keywords: string[];
  hasObjectStorePath: boolean;
  error?: string;
}

interface RunResult {
  ranAt: number;
  status: "loading" | "done" | "error";
  containerIds: number[];
  insights: ResolvedInsight[];
  errorMessage?: string;
}

/** Same bridge-first/fetch-fallback convention as ozoneClient.ts's postHttp
 * (not exported there, so reproduced locally rather than duplicating the
 * whole module or reaching into its internals). */
async function postReviewPass(): Promise<{ insight_container_ids: number[] }> {
  const oz = (window as any).ozone;
  if (oz?.http?.post) {
    return oz.http.post("/consciousness/review_pass", {});
  }
  const host = (window as any).OZONE_HOST_URL || "http://127.0.0.1:50051";
  const headers: Record<string, string> = { "Content-Type": "application/json" };
  try {
    const token = localStorage.getItem("ozone_session_token");
    if (token) headers["Authorization"] = `Bearer ${token}`;
  } catch {
    /* localStorage unavailable — proceed unauthenticated, same as ozoneClient.ts */
  }
  const res = await fetch(`${host}/consciousness/review_pass`, {
    method: "POST",
    headers,
    body: JSON.stringify({}),
  });
  if (!res.ok) throw new Error(`review_pass → HTTP ${res.status}`);
  return res.json();
}

function kindFromContainer(c: RawContainer): string {
  const fromKeyword = c.local_state.context.keywords.find((k) => k !== "review-pass");
  if (fromKeyword) return fromKeyword;
  const name = c.local_state.metadata.name ?? "";
  const prefix = "Consciousness insight: ";
  return name.startsWith(prefix) ? name.slice(prefix.length) : name || "(unknown kind)";
}

async function resolveInsight(containerId: number): Promise<ResolvedInsight> {
  const result = await zseiQuery<{ Container?: RawContainer }>({ GetContainer: { container_id: containerId } });
  const c = result?.Container;
  if (!c) {
    return {
      containerId,
      kind: "(unresolved)",
      name: "",
      materializedPath: "",
      provenance: "",
      createdAt: 0,
      keywords: [],
      hasObjectStorePath: false,
      error: "GetContainer returned no container for this id.",
    };
  }
  return {
    containerId,
    kind: kindFromContainer(c),
    name: c.local_state.metadata.name ?? "",
    materializedPath: c.local_state.metadata.materialized_path ?? "",
    provenance: c.local_state.metadata.provenance,
    createdAt: c.local_state.metadata.created_at,
    keywords: c.local_state.context.keywords,
    hasObjectStorePath: !!c.local_state.storage.object_store_path,
  };
}

function formatTime(unixSeconds: number): string {
  if (!unixSeconds) return "—";
  return new Date(unixSeconds * 1000).toLocaleString();
}

const C_TEXT = "var(--color-text)";
const C_BODY = "var(--color-text-secondary)";
const C_MUTED = "var(--color-text-muted)";
const C_BORDER = "var(--color-border-faint)";
const C_WARN = "#e8c14f";
const C_GOOD = "#8fe38f";

export const ConsciousnessReview: React.FC<ConsciousnessReviewProps> = () => {
  const [runs, setRuns] = useState<RunResult[]>([]);
  const running = runs.length > 0 && runs[0].status === "loading";

  async function trigger() {
    const started: RunResult = { ranAt: Date.now(), status: "loading", containerIds: [], insights: [] };
    setRuns((prev) => [started, ...prev]);
    try {
      const { insight_container_ids } = await postReviewPass();
      const insights = await Promise.all(insight_container_ids.map(resolveInsight));
      setRuns((prev) =>
        prev.map((r) => (r === started ? { ...r, status: "done", containerIds: insight_container_ids, insights } : r)),
      );
    } catch (e) {
      setRuns((prev) =>
        prev.map((r) =>
          r === started ? { ...r, status: "error", errorMessage: e instanceof Error ? e.message : String(e) } : r,
        ),
      );
    }
  }

  return (
    <div style={{ fontSize: 12.5, color: C_BODY, lineHeight: 1.6 }}>
      <div style={{ fontSize: 12.5, fontWeight: 700, color: C_TEXT, marginBottom: 6 }}>
        Consciousness review pass
      </div>
      <p style={{ margin: "0 0 10px", color: C_MUTED }}>
        Runs a real, model-backed self-review over the decision-review capture store and recent tasks, and persists
        any genuine finding as a real container. This is manually triggered, not a background loop. A run that
        returns zero insights is a correct, honest outcome — it means nothing this pass genuinely warranted
        flagging, not that the button failed.
      </p>
      <button
        onClick={trigger}
        disabled={running}
        style={{
          background: running ? "var(--color-border-faint)" : "#25314a",
          color: C_TEXT,
          border: `1px solid ${C_BORDER}`,
          borderRadius: 6,
          padding: "6px 14px",
          fontSize: 12.5,
          cursor: running ? "default" : "pointer",
        }}
      >
        {running ? "Running review pass…" : "Run review pass"}
      </button>

      <div style={{ marginTop: 16, display: "flex", flexDirection: "column", gap: 12 }}>
        {runs.length === 0 && (
          <div style={{ color: C_MUTED, fontStyle: "italic" }}>No review pass has been run this session yet.</div>
        )}
        {runs.map((run) => (
          <div key={run.ranAt} style={{ border: `1px solid ${C_BORDER}`, borderRadius: 8, padding: "8px 10px" }}>
            <div style={{ display: "flex", justifyContent: "space-between", color: C_MUTED, fontSize: 11 }}>
              <span>{new Date(run.ranAt).toLocaleTimeString()}</span>
              <span>
                {run.status === "loading" && "running…"}
                {run.status === "error" && <span style={{ color: "#ff8a8a" }}>error</span>}
                {run.status === "done" &&
                  (run.containerIds.length === 0 ? (
                    <span style={{ color: C_MUTED }}>0 insights (correct outcome — nothing flagged)</span>
                  ) : (
                    <span style={{ color: C_GOOD }}>{run.containerIds.length} insight(s)</span>
                  ))}
              </span>
            </div>
            {run.status === "error" && (
              <div style={{ color: "#ff8a8a", fontSize: 12, marginTop: 4 }}>{run.errorMessage}</div>
            )}
            {run.insights.map((ins) => (
              <div
                key={ins.containerId}
                style={{ marginTop: 8, paddingTop: 8, borderTop: `1px solid ${C_BORDER}`, fontSize: 12 }}
              >
                <div>
                  <b>{ins.kind}</b>{" "}
                  <span style={{ color: C_MUTED }}>(container {ins.containerId})</span>
                </div>
                {ins.error ? (
                  <div style={{ color: "#ff8a8a" }}>{ins.error}</div>
                ) : (
                  <>
                    <div style={{ color: C_MUTED }}>{ins.materializedPath}</div>
                    <div style={{ color: C_MUTED }}>
                      created {formatTime(ins.createdAt)} · provenance: {ins.provenance}
                    </div>
                    <div style={{ marginTop: 4, color: C_WARN, fontSize: 11 }}>
                      This insight's real content/citations text is persisted on disk but is not reachable through
                      any current read endpoint — it's written as extra fields outside the container's typed schema,
                      and it has no <code>object_store_path</code> file for <code>GetContainerContent</code> to read
                      either (confirmed on this container: {ins.hasObjectStorePath ? "path present" : "no path set"}
                      ). A backend fix would need a raw-JSON read path for this container type.
                    </div>
                  </>
                )}
              </div>
            ))}
          </div>
        ))}
      </div>
    </div>
  );
};

export default ConsciousnessReview;
