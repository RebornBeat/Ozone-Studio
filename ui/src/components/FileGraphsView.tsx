/**
 * FileGraphsView — the graphs the host created BY ITSELF for linked files (operator 2026-11-11: "see the call graphs even without running a prompt",
 * "every modality should have something like a call graph working with entrypoints").
 *
 * Source: GET /callgraph/files — one row per linked file written by `orchestrator/graph_on_link.rs` when a file was linked, when a linked file changed,
 * and once at workspace load: modality, the ModalityGraph container, node/edge counts, the graph's ENTRIES and the state of the `wo_graph` task that did
 * the work. For code the entries are the explicit entry points the code pipeline recorded; for every other modality they are the graph's roots. The
 * legend above the list says, per modality, what an entry means. Nothing is scored or ranked; a failed step shows its error and the task stays open
 * (the next trigger for the same content resumes it).
 */
import React, { useEffect, useMemo, useState } from "react";
import { fetchFileGraphs, type FileGraphRow, type FileGraphsResponse } from "../ozoneClient";

export interface FileGraphsViewProps {
  projectId?: number | null;
  /** show only this modality (used inside a modality's own view) */
  modality?: string;
  /** collapse behind a summary */
  compact?: boolean;
  /** milliseconds between refreshes (default 5000) */
  pollMs?: number;
}

const STATUS_TEXT: Record<string, string> = {
  complete: "graph created, entries read",
  graph_created: "graph created, entries pending",
  failed_analyze: "analysis failed — will resume on the next trigger",
  failed_create_graph: "graph creation failed — will resume on the next trigger",
  not_text_readable: "not readable as text — no graph (see the note)",
  file_unreadable: "file missing or unreadable",
};

function when(at: number): string {
  return at ? new Date(at * 1000).toLocaleString() : "";
}

const FileGraphsView: React.FC<FileGraphsViewProps> = ({ projectId, modality, compact, pollMs = 5000 }) => {
  const [data, setData] = useState<FileGraphsResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [pick, setPick] = useState<string>(modality ?? "all");

  useEffect(() => {
    let alive = true;
    const load = () =>
      fetchFileGraphs(projectId ?? null)
        .then((r) => { if (alive) { setData(r); setError(null); } })
        .catch((e) => { if (alive) setError(String(e?.message ?? e)); });
    load();
    const t = window.setInterval(load, pollMs);
    return () => { alive = false; window.clearInterval(t); };
  }, [projectId, pollMs]);

  const rows: FileGraphRow[] = useMemo(() => {
    const all = data?.files ?? [];
    const want = modality ?? (pick === "all" ? null : pick);
    return want ? all.filter((r) => r.modality === want) : all;
  }, [data, modality, pick]);
  const modalities = useMemo(() => Array.from(new Set((data?.files ?? []).map((r) => r.modality))).sort(), [data]);
  const concept = (m: string) => data?.entry_concepts.find((c) => c.modality === m);

  const body = (
    <div className="ocg-files">
      {error && <div className="ocg-note">Could not read /callgraph/files: {error}</div>}
      {data && !data.enabled && <div className="ocg-note">Graph creation on file link is switched off on this host (OZONE_GRAPH_ON_LINK).</div>}
      {!modality && modalities.length > 1 && (
        <div className="ocg-tabs">
          <button className={`ocg-tab${pick === "all" ? " ocg-tab-on" : ""}`} onClick={() => setPick("all")}>all</button>
          {modalities.map((m) => (
            <button key={m} className={`ocg-tab${pick === m ? " ocg-tab-on" : ""}`} onClick={() => setPick(m)}>{m}</button>
          ))}
        </div>
      )}
      {data && rows.length === 0 && (
        <div className="ocg-note">
          No linked file has a graph yet{modality ? ` for ${modality}` : ""}. A graph is created automatically when a file is linked, when a linked file changes, and once when the workspace loads.
        </div>
      )}
      <ul className="ocg-sites">
        {rows.map((r) => {
          const c = concept(r.modality);
          return (
            <li key={`${r.file_reference_id}-${r.hash}`} className={`ocg-site${r.status === "complete" ? " ocg-site-ran" : ""}`}>
              <span className="ocg-kind">{r.modality}</span>
              <span className="ocg-label omono">{r.path}</span>
              <span className="ocg-dim">{STATUS_TEXT[r.status] ?? r.status}</span>
              {r.task_status && <span className="ocg-badge" title="the wo_graph work-order task for this file's content">task {r.task_id}: {r.task_status}</span>}
              {typeof r.nodes === "number" && <span className="ocg-dim">{r.nodes} nodes · {r.edges ?? 0} edges</span>}
              <span className="ocg-dim">{r.trigger} · {when(r.at)}</span>
              {r.entries && r.entries.length > 0 && (
                <div className="ocg-dim" title={c?.entry}>
                  {c?.explicit_entry_points ? "entry points" : "entries (roots)"}: <span className="omono">{r.entries.join(", ")}</span>
                </div>
              )}
              {r.entries && r.entries.length === 0 && r.status === "complete" && <div className="ocg-dim">no entries found in this graph</div>}
              {(r.error || r.note) && <div className="ocg-note">{r.error ?? r.note}</div>}
            </li>
          );
        })}
      </ul>
      {modality && concept(modality) && <div className="ocg-note">An entry for {modality}: {concept(modality)!.entry}</div>}
    </div>
  );

  return compact ? (
    <details className="ocg-stage">
      <summary>
        <span className="ocg-label">Graphs created for linked {modality ?? ""} files</span>
        <span className="ocg-dim">{rows.length} file{rows.length === 1 ? "" : "s"}</span>
      </summary>
      {body}
    </details>
  ) : (
    <div className="ocg-section">
      <div className="ocg-title">Graphs created for linked files</div>
      {body}
    </div>
  );
};

export default FileGraphsView;
