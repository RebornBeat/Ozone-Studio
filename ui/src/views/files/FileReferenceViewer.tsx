/**
 * F2 — File/URL/Package reference viewer.
 *
 * Shows the real ZSEI container behind the reference currently selected in
 * F1's Workspace Browser (`useSelectedFile`, ../../fileSelection). Ground
 * truth read directly from
 * assets/pipelines/general/{file_link,url_link,package_link}/main.rs:
 *
 * - The container itself (`link_reference_to_graph` in each pipeline) has
 *   NO dedicated path/url/registry/version fields — those live only in each
 *   pipeline's flat `*RefInfo` JSON (files_<project>.json etc.), which this
 *   view has no route to read. The one place the real identity survives
 *   into the container is `local_state.metadata.name`, built as:
 *     file:    `format!("File: {}", file_ref.path)`
 *     url:     `format!("URL: {}", url_ref.url)`
 *     package: `format!("Package: {}/{}", registry, name)`
 *   so this view parses that prefix rather than guessing a dedicated field
 *   exists. `selectedFile.path` (when F1 supplies it) is used as a second,
 *   preferred source for the file path.
 * - `metadata.modality` is hardcoded `"Unknown"` at creation time in all
 *   three pipelines — never displayed as if it were real.
 * - `metadata.provenance` (e.g. "file_link_pipeline"), `created_at`,
 *   `updated_at` (unix seconds) are real and shown.
 * - `context.keywords` real; `context.relationships` real but best-effort —
 *   `link_relation_both_ways` writes `Contains` (project→ref) and `PartOf`
 *   (ref→project), confidence 1.0, discovered_via "Manual" — but silently
 *   skips a direction if that side's GetContainer call failed, so an
 *   incomplete relation set is expected, not an error.
 * - Package `version` is NOT recoverable here: `PackageRefInfo.version`
 *   lives only in the flat store, never written into the container or its
 *   name string (only `registry/name` is). Stated plainly, not guessed.
 * - Linked analysis (`file_link`'s `analyze_file`) is written to a FLAT file
 *   at `{OZONE_ZSEI_PATH}/local/file_analysis/{project_id}_{file_ref_id}.json`
 *   — a different env var than `OZONE_ZSEI_DATA_DIR` that `GetContainerContent`
 *   resolves against, no `object_store_path`, no host route reads it. This
 *   view says so rather than attempting (and failing) to fetch it.
 */
import React, { useEffect, useState } from "react";
import { zseiQuery } from "../../ozoneClient";
import { useSelectedFile } from "../../fileSelection";
import { readFileContent, FileContentUnavailable, type FileContentResult } from "../../data/fileContent";

const C_TEXT = "#dfe7f2";
const C_BODY = "#c7d0dc";
const C_MUTED = "#8b98ab";
const C_BORDER = "#1e2836";
const C_WARN = "#e8c14f";
const C_ERR = "#ff8a8a";

const wrap: React.CSSProperties = { overflowWrap: "anywhere", wordBreak: "break-word", minWidth: 0 };

interface ContainerRelationRaw {
  target_id: number;
  relation_type: string;
  confidence?: number;
  discovered_via?: string;
}
interface ContainerJson {
  global_state: { container_id: number; parent_id: number; child_ids: number[] };
  local_state: {
    metadata: {
      container_type: string;
      modality: string;
      name?: string | null;
      provenance?: string;
      created_at?: number;
      updated_at?: number;
      materialized_path?: string | null;
    };
    context: { keywords: string[]; relationships: ContainerRelationRaw[] };
    storage: { object_store_path?: string | null };
  };
}

function unwrapContainer(result: unknown): ContainerJson | null {
  if (result && typeof result === "object" && "Container" in (result as Record<string, unknown>)) {
    return (result as { Container: ContainerJson }).Container;
  }
  return null;
}

/** Parses the real `"File: <path>"` / `"URL: <url>"` / `"Package: <registry>/<name>"`
 * name convention each linking pipeline writes (see file header) — the only place
 * the reference's real identity survives into the container itself. */
function parseNamePrefix(name: string | null | undefined): { prefix: string | null; rest: string | null } {
  if (!name) return { prefix: null, rest: null };
  const m = name.match(/^(File|URL|Package):\s(.*)$/s);
  return m ? { prefix: m[1], rest: m[2] } : { prefix: null, rest: null };
}

function formatTs(secs: number | undefined): string | null {
  if (typeof secs !== "number" || secs <= 0) return null;
  try {
    return new Date(secs * 1000).toLocaleString();
  } catch {
    return null;
  }
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(2)} MB`;
}

const SectionTitle: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <div
    style={{
      fontSize: 11,
      fontWeight: 700,
      letterSpacing: 0.4,
      textTransform: "uppercase",
      color: C_MUTED,
      margin: "14px 0 6px",
      paddingTop: 10,
      borderTop: `1px solid ${C_BORDER}`,
    }}
  >
    {children}
  </div>
);
const Row: React.FC<{ label: string; children: React.ReactNode }> = ({ label, children }) => (
  <div style={{ ...wrap, marginBottom: 3, fontSize: 12.5 }}>
    <span style={{ color: C_MUTED }}>{label}: </span>
    <span style={{ color: C_BODY }}>{children}</span>
  </div>
);
const Empty: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <div style={{ ...wrap, fontSize: 11.5, color: C_MUTED, fontStyle: "italic" }}>{children}</div>
);

/** Resolves relation target ids to a short label (real container name/type), best-effort. */
function useRelationLabels(relations: ContainerRelationRaw[]): Record<number, string> {
  const [labels, setLabels] = useState<Record<number, string>>({});
  useEffect(() => {
    let cancelled = false;
    setLabels({});
    const ids = relations.map((r) => r.target_id).slice(0, 20);
    (async () => {
      const out: Record<number, string> = {};
      for (const id of ids) {
        try {
          const c = unwrapContainer(await zseiQuery<any>({ GetContainer: { container_id: id } }));
          if (!c) continue;
          const parsed = parseNamePrefix(c.local_state.metadata.name);
          out[id] = parsed.rest ?? c.local_state.metadata.name ?? `${c.local_state.metadata.container_type} #${id}`;
        } catch {
          /* leave unresolved — id alone is still shown */
        }
      }
      if (!cancelled) setLabels(out);
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [relations.map((r) => r.target_id).join(",")]);
  return labels;
}

const FileContentBlock: React.FC<{ path: string }> = ({ path }) => {
  const [state, setState] = useState<
    { kind: "loading" } | { kind: "error"; message: string } | { kind: "ready"; result: FileContentResult }
  >({ kind: "loading" });

  useEffect(() => {
    let cancelled = false;
    setState({ kind: "loading" });
    readFileContent(path)
      .then((result) => !cancelled && setState({ kind: "ready", result }))
      .catch((e) => {
        if (cancelled) return;
        const message =
          e instanceof FileContentUnavailable
            ? "File content access requires the desktop app and hasn't been enabled for this build yet."
            : e instanceof Error
              ? e.message
              : String(e);
        setState({ kind: "error", message });
      });
    return () => {
      cancelled = true;
    };
  }, [path]);

  if (state.kind === "loading") return <Empty>Loading file content…</Empty>;
  if (state.kind === "error")
    return (
      <div style={{ ...wrap, fontSize: 11.5, color: C_WARN }}>
        Content not shown: {state.message}
      </div>
    );
  const { result } = state;
  return (
    <div>
      <div style={{ fontSize: 11, color: C_MUTED, marginBottom: 4 }}>
        {formatBytes(result.sizeBytes)}
        {result.truncated ? " · truncated" : ""}
      </div>
      <pre
        style={{
          ...wrap,
          maxHeight: 320,
          overflow: "auto",
          background: "#0a0f1a",
          border: `1px solid ${C_BORDER}`,
          borderRadius: 6,
          padding: "8px 10px",
          fontSize: 11.5,
          color: C_BODY,
          margin: 0,
        }}
      >
        {result.content}
      </pre>
    </div>
  );
};

export type FileReferenceViewerProps = { projectId: number | null };

export const FileReferenceViewer: React.FC<FileReferenceViewerProps> = () => {
  const [selected] = useSelectedFile();
  const [status, setStatus] = useState<
    { kind: "loading" } | { kind: "error"; message: string } | { kind: "ready"; container: ContainerJson }
  >({ kind: "loading" });

  useEffect(() => {
    if (!selected) return;
    let cancelled = false;
    setStatus({ kind: "loading" });
    zseiQuery<any>({ GetContainer: { container_id: selected.containerId } })
      .then((result) => {
        if (cancelled) return;
        const container = unwrapContainer(result);
        if (!container) {
          setStatus({ kind: "error", message: `Container ${selected.containerId} not found` });
        } else {
          setStatus({ kind: "ready", container });
        }
      })
      .catch((e) => !cancelled && setStatus({ kind: "error", message: e instanceof Error ? e.message : String(e) }));
    return () => {
      cancelled = true;
    };
  }, [selected]);

  // Called unconditionally (rules of hooks) — empty array when there's nothing to resolve yet.
  const relations = status.kind === "ready" ? status.container.local_state.context.relationships ?? [] : [];
  const relationLabels = useRelationLabels(relations);

  if (!selected) {
    return (
      <div style={{ padding: 16, fontSize: 12.5, color: C_MUTED }}>
        Nothing selected — pick a file, URL, or package reference from the Browser sub-tab.
      </div>
    );
  }

  if (status.kind === "loading") return <div style={{ padding: 16, fontSize: 12.5, color: C_MUTED }}>Loading reference…</div>;
  if (status.kind === "error")
    return <div style={{ padding: 16, fontSize: 12.5, color: C_ERR }}>Error: {status.message}</div>;

  const { container } = status;
  const meta = container.local_state.metadata;
  const { prefix, rest } = parseNamePrefix(meta.name);
  const keywords = container.local_state.context.keywords ?? [];
  const created = formatTs(meta.created_at);
  const updated = formatTs(meta.updated_at);

  const filePath = selected.kind === "file" ? selected.path ?? (prefix === "File" ? rest ?? undefined : undefined) : undefined;
  const url = selected.kind === "url" ? (prefix === "URL" ? rest : null) : null;
  const pkg = selected.kind === "package" ? (prefix === "Package" ? rest : null) : null;

  return (
    <div style={{ padding: 4, fontSize: 12.5, color: C_BODY, lineHeight: 1.6 }}>
      <div style={{ fontSize: 14, fontWeight: 700, color: C_TEXT, ...wrap }}>
        {selected.name || meta.name || `Container #${container.global_state.container_id}`}
      </div>

      <Row label="Kind">{meta.container_type} (container #{container.global_state.container_id})</Row>
      {meta.provenance && <Row label="Linked via">{meta.provenance}</Row>}
      {created && <Row label="Created">{created}</Row>}
      {updated && updated !== created && <Row label="Updated">{updated}</Row>}
      {keywords.length > 0 && <Row label="Keywords">{keywords.join(", ")}</Row>}
      <Row label="Modality">
        {meta.modality === "Unknown" ? (
          <span style={{ color: C_MUTED, fontStyle: "italic" }}>not recorded (always "Unknown" at creation time)</span>
        ) : (
          meta.modality
        )}
      </Row>

      {selected.kind === "file" && (
        <>
          <SectionTitle>File</SectionTitle>
          {filePath ? <Row label="Path">{filePath}</Row> : <Empty>No path recorded for this reference.</Empty>}
          {filePath && <FileContentBlock path={filePath} />}
          <div style={{ marginTop: 10, fontSize: 11, color: C_MUTED, ...wrap }}>
            Linked analysis (if any) is written to a flat file outside the graph store this view can read
            ({"{OZONE_ZSEI_PATH}"}/local/file_analysis/…) — not shown here; that is a real backend gap, not a
            loading failure.
          </div>
        </>
      )}

      {selected.kind === "url" && (
        <>
          <SectionTitle>URL</SectionTitle>
          {url ? <Row label="Address">{url}</Row> : <Empty>No URL recorded for this reference.</Empty>}
        </>
      )}

      {selected.kind === "package" && (
        <>
          <SectionTitle>Package</SectionTitle>
          {pkg ? <Row label="Registry / name">{pkg}</Row> : <Empty>No package identity recorded for this reference.</Empty>}
          <Empty>Version is not recoverable here — it is stored only in a flat file this view has no route to read.</Empty>
        </>
      )}

      <SectionTitle>Graph relationships (best-effort)</SectionTitle>
      {relations.length === 0 ? (
        <Empty>None recorded — the bidirectional link write is best-effort and may not have completed.</Empty>
      ) : (
        relations.map((r, i) => (
          <Row key={i} label={r.relation_type}>
            {relationLabels[r.target_id] ?? `container #${r.target_id}`}
            {typeof r.confidence === "number" && ` (confidence ${r.confidence.toFixed(2)})`}
            {r.discovered_via && <span style={{ color: C_MUTED }}> — {r.discovered_via}</span>}
          </Row>
        ))
      )}
    </div>
  );
};

export default FileReferenceViewer;
