/**
 * C11 — Node detail panel (Graph View sidebar).
 *
 * Renders only what genuinely exists on the selected node's untouched backend
 * payload (`node.raw`). Field sources, confirmed against the pipeline structs:
 *   code  — CodeGraphNode  (assets/pipelines/modalities/code/main.rs):
 *           name, position{file_path,start_line,end_line,start_column?,end_column?}
 *   math  — MathGraphNode  (.../math/main.rs): label, content, step_number,
 *           confidence, annotations[]
 *   text  — TextGraphNode  (.../text/main.rs): content, position{start_offset,
 *           end_offset,line?,column?}, semantic_annotations[], provisional,
 *           provisional_status, provenance, materialized_path, keywords[],
 *           hotness_score, version, version_notes[], source_* refs,
 *           cross_modal_refs[]
 * A field that is absent, null, or an empty array is simply not shown — never
 * a placeholder like 0 / "N/A".
 *
 * "Graph container relationships" (B16) are the Relation[] of the whole
 * ModalityGraph CONTAINER this node was loaded from — container-to-container
 * links (SimilarTo, ForkOf, ...), NOT edges incident to this node — and are
 * labelled as such.
 */
import React, { useState } from "react";
import type { ContainerRelation, GraphViewNode } from "../graphViewTypes";

export interface NodeDetailPanelProps {
  node: GraphViewNode | null;
  /** Real `local_state.context.relationships` per ModalityGraph container id (B16). */
  containerRelations: Record<number, ContainerRelation[]>;
}

const C_TEXT = "var(--color-text)";
const C_BODY = "var(--color-text-secondary)";
const C_MUTED = "var(--color-text-muted)";
const C_BORDER = "var(--color-border-faint)";
const C_WARN = "#e8c14f";
const C_PANEL = "var(--color-bg)";

const wrap: React.CSSProperties = { overflowWrap: "anywhere", wordBreak: "break-word", minWidth: 0 };

const scrollBox: React.CSSProperties = {
  ...wrap,
  maxHeight: 160,
  overflowY: "auto",
  whiteSpace: "pre-wrap",
  background: C_PANEL,
  border: `1px solid ${C_BORDER}`,
  borderRadius: 6,
  padding: "6px 8px",
  fontSize: 11.5,
  color: C_BODY,
  lineHeight: 1.5,
};

function isPresent(v: unknown): boolean {
  if (v === undefined || v === null) return false;
  if (Array.isArray(v)) return v.length > 0;
  return true;
}

function formatScalar(v: unknown): string {
  if (typeof v === "string") return v;
  if (typeof v === "number" || typeof v === "boolean") return String(v);
  return JSON.stringify(v);
}

/** Code positions carry file_path/start_line/...; text positions carry start_offset/.... */
function formatPosition(p: unknown): string | null {
  if (!p || typeof p !== "object") return null;
  const o = p as Record<string, unknown>;
  if (typeof o.file_path === "string") {
    let s = `${o.file_path}:${o.start_line ?? "?"}`;
    if (o.end_line !== undefined && o.end_line !== o.start_line) s += `–${o.end_line}`;
    if (typeof o.start_column === "number") {
      s += ` (col ${o.start_column}${typeof o.end_column === "number" ? `–${o.end_column}` : ""})`;
    }
    return s;
  }
  if (typeof o.start_offset === "number") {
    let s = `chars ${o.start_offset}–${o.end_offset ?? "?"}`;
    if (typeof o.line === "number") s += `, line ${o.line}`;
    if (typeof o.column === "number") s += `, col ${o.column}`;
    return s;
  }
  return JSON.stringify(p);
}

/** Rust enum serialization: unit variants are strings, tuple variants are {Variant: payload}. */
function formatEnumish(v: unknown): string {
  if (v && typeof v === "object" && !Array.isArray(v)) {
    const entries = Object.entries(v as Record<string, unknown>);
    if (entries.length === 1) return `${entries[0][0]}(${formatScalar(entries[0][1])})`;
  }
  return formatScalar(v);
}

const SectionTitle: React.FC<{ children: React.ReactNode; title?: string }> = ({ children, title }) => (
  <div
    title={title}
    style={{
      fontSize: 11,
      fontWeight: 700,
      letterSpacing: 0.4,
      textTransform: "uppercase",
      color: C_MUTED,
      margin: "12px 0 4px",
      paddingTop: 8,
      borderTop: `1px solid ${C_BORDER}`,
    }}
  >
    {children}
  </div>
);

const Row: React.FC<{ label: string; children: React.ReactNode }> = ({ label, children }) => (
  <div style={{ ...wrap, marginBottom: 2 }}>
    <span style={{ color: C_MUTED }}>{label}: </span>
    <span style={{ color: C_BODY }}>{children}</span>
  </div>
);

interface Annotation {
  annotation_type?: unknown;
  value?: unknown;
  confidence?: unknown;
  source?: unknown;
}

const Annotations: React.FC<{ label: string; items: unknown[] }> = ({ label, items }) => (
  <div style={{ marginTop: 4 }}>
    <div style={{ color: C_MUTED }}>{label} ({items.length}):</div>
    {items.map((raw, i) => {
      const a = (raw && typeof raw === "object" ? raw : {}) as Annotation;
      const known = typeof a.annotation_type === "string" || typeof a.value === "string";
      return (
        <div key={i} style={{ ...wrap, margin: "2px 0 2px 8px", color: C_BODY }}>
          {known ? (
            <>
              <b>{formatScalar(a.annotation_type)}</b>: {formatScalar(a.value)}
              {typeof a.confidence === "number" && ` (${a.confidence.toFixed(2)})`}
              {typeof a.source === "string" && <span style={{ color: C_MUTED }}> — {a.source}</span>}
            </>
          ) : (
            JSON.stringify(raw)
          )}
        </div>
      );
    })}
  </div>
);

const NodeBody: React.FC<{ node: GraphViewNode; relations: ContainerRelation[] | undefined }> = ({
  node,
  relations,
}) => {
  const [showRaw, setShowRaw] = useState(false);
  const raw = node.raw as Record<string, unknown>;
  const properties =
    raw.properties && typeof raw.properties === "object" && !Array.isArray(raw.properties)
      ? (raw.properties as Record<string, unknown>)
      : {};
  const propEntries = Object.entries(properties);
  const position = formatPosition(raw.position);
  const rawContent = typeof raw.content === "string" ? raw.content : undefined;

  return (
    <div style={{ fontSize: 12, color: C_BODY, lineHeight: 1.6 }}>
      {!node.isReal && (
        <div
          style={{
            ...wrap,
            border: `1px solid ${C_WARN}`,
            color: C_WARN,
            borderRadius: 6,
            padding: "6px 8px",
            marginBottom: 8,
            fontSize: 11.5,
          }}
        >
          Schema-only type — no construction site produces this in the current pipelines.
        </div>
      )}

      <Row label="Modality">{node.modality}</Row>
      <Row label="Type">{node.nodeType}</Row>
      <Row label="Label">{node.label}</Row>
      {typeof raw.node_id === "number" && <Row label="Node id">{raw.node_id}</Row>}
      {typeof node.confidence === "number" && <Row label="Confidence">{node.confidence.toFixed(2)}</Row>}
      <Row label="Source container">{node.sourceContainerId}</Row>

      {node.contentPreview && (
        <>
          <SectionTitle>Content</SectionTitle>
          <div style={scrollBox}>{node.contentPreview}</div>
        </>
      )}

      <SectionTitle>{node.modality} fields</SectionTitle>
      {node.modality === "code" && (
        <>
          {isPresent(raw.name) && <Row label="Name">{formatScalar(raw.name)}</Row>}
          {position && <Row label="Position">{position}</Row>}
        </>
      )}
      {node.modality === "math" && (
        <>
          {isPresent(raw.label) && <Row label="Label">{formatScalar(raw.label)}</Row>}
          {isPresent(raw.step_number) && <Row label="Step number">{formatScalar(raw.step_number)}</Row>}
          {rawContent !== undefined && rawContent !== node.contentPreview && (
            <>
              <div style={{ color: C_MUTED }}>Raw content:</div>
              <div style={scrollBox}>{rawContent}</div>
            </>
          )}
          {Array.isArray(raw.annotations) && raw.annotations.length > 0 && (
            <Annotations label="Annotations" items={raw.annotations} />
          )}
        </>
      )}
      {node.modality === "text" && (
        <>
          {isPresent(raw.materialized_path) && <Row label="Materialized path">{formatScalar(raw.materialized_path)}</Row>}
          {Array.isArray(raw.keywords) && raw.keywords.length > 0 && (
            <Row label="Keywords">{raw.keywords.map(formatScalar).join(", ")}</Row>
          )}
          {typeof raw.provisional === "boolean" && <Row label="Provisional">{raw.provisional ? "yes" : "no"}</Row>}
          {isPresent(raw.provisional_status) && <Row label="Provisional status">{formatEnumish(raw.provisional_status)}</Row>}
          {isPresent(raw.provenance) && <Row label="Provenance">{formatEnumish(raw.provenance)}</Row>}
          {typeof raw.hotness_score === "number" && <Row label="Hotness">{raw.hotness_score.toFixed(2)}</Row>}
          {position && <Row label="Position">{position}</Row>}
          {isPresent(raw.version) && <Row label="Version">{formatScalar(raw.version)}</Row>}
          {isPresent(raw.source_file_id) && <Row label="Source file id">{formatScalar(raw.source_file_id)}</Row>}
          {isPresent(raw.source_chunk_id) && <Row label="Source chunk id">{formatScalar(raw.source_chunk_id)}</Row>}
          {isPresent(raw.source_chunk_index) && <Row label="Source chunk index">{formatScalar(raw.source_chunk_index)}</Row>}
          {isPresent(raw.source_start_char) && (
            <Row label="Source chars">
              {formatScalar(raw.source_start_char)}–{isPresent(raw.source_end_char) ? formatScalar(raw.source_end_char) : "?"}
            </Row>
          )}
          {isPresent(raw.created_by_step) && <Row label="Created by step">{formatScalar(raw.created_by_step)}</Row>}
          {isPresent(raw.updated_by_step) && <Row label="Updated by step">{formatScalar(raw.updated_by_step)}</Row>}
          {Array.isArray(raw.semantic_annotations) && raw.semantic_annotations.length > 0 && (
            <Annotations label="Semantic annotations" items={raw.semantic_annotations} />
          )}
          {Array.isArray(raw.cross_modal_refs) && raw.cross_modal_refs.length > 0 && (
            <div style={{ marginTop: 4 }}>
              <div style={{ color: C_MUTED }}>Cross-modal refs ({raw.cross_modal_refs.length}):</div>
              {raw.cross_modal_refs.map((r, i) => (
                <div key={i} style={{ ...wrap, margin: "2px 0 2px 8px" }}>{JSON.stringify(r)}</div>
              ))}
            </div>
          )}
          {Array.isArray(raw.version_notes) && raw.version_notes.length > 0 && (
            <div style={{ marginTop: 4 }}>
              <div style={{ color: C_MUTED }}>Version notes ({raw.version_notes.length}):</div>
              {raw.version_notes.map((n, i) => {
                const o = (n && typeof n === "object" ? n : {}) as Record<string, unknown>;
                return (
                  <div key={i} style={{ ...wrap, margin: "2px 0 2px 8px" }}>
                    {typeof o.version === "number" ? `v${o.version}: ` : ""}
                    {typeof o.note === "string" ? o.note : JSON.stringify(n)}
                  </div>
                );
              })}
            </div>
          )}
        </>
      )}

      <SectionTitle>Properties</SectionTitle>
      {propEntries.length === 0 ? (
        <div style={{ color: C_MUTED }}>No properties recorded.</div>
      ) : (
        <details open={propEntries.length <= 8}>
          <summary style={{ cursor: "pointer", color: C_TEXT }}>
            {propEntries.length} propert{propEntries.length === 1 ? "y" : "ies"}
          </summary>
          <div style={{ marginTop: 4 }}>
            {propEntries.map(([k, v]) => (
              <Row key={k} label={k}>{formatScalar(v)}</Row>
            ))}
          </div>
        </details>
      )}

      <SectionTitle title="Container-level relationships of the whole ModalityGraph container this node was loaded from (B16). These are not edges of this node.">
        Graph container relationships
      </SectionTitle>
      <div style={{ fontSize: 11, color: C_MUTED, marginBottom: 4 }}>
        Links of graph container #{node.sourceContainerId} as a whole — not edges of this node.
      </div>
      {!relations || relations.length === 0 ? (
        <div style={{ color: C_MUTED }}>None recorded.</div>
      ) : (
        relations.map((r, i) => (
          <div
            key={`${r.relation_type}-${r.target_id}-${i}`}
            style={{ ...wrap, border: `1px solid ${C_BORDER}`, borderRadius: 6, padding: "4px 8px", marginBottom: 4 }}
          >
            <div style={{ color: C_TEXT }}>
              <b>{r.relation_type}</b> → #{r.target_id}
            </div>
            <div style={{ color: C_MUTED, fontSize: 11 }}>
              {typeof r.confidence === "number" && `confidence ${r.confidence.toFixed(2)} · `}
              via {r.discovered_via}
              {typeof r.graph_hops === "number" && ` · ${r.graph_hops} hop${r.graph_hops === 1 ? "" : "s"}`}
            </div>
          </div>
        ))
      )}

      <div style={{ marginTop: 12 }}>
        <button
          type="button"
          onClick={() => setShowRaw((s) => !s)}
          style={{
            background: "#101724",
            color: C_TEXT,
            border: `1px solid ${C_BORDER}`,
            borderRadius: 6,
            padding: "3px 10px",
            fontSize: 11.5,
            cursor: "pointer",
          }}
        >
          {showRaw ? "Hide raw JSON" : "Show raw JSON"}
        </button>
        {showRaw && (
          <pre style={{ ...scrollBox, maxHeight: 240, marginTop: 6, overflowX: "auto", whiteSpace: "pre-wrap" }}>
            {JSON.stringify(node.raw, null, 2)}
          </pre>
        )}
      </div>
    </div>
  );
};

export const NodeDetailPanel: React.FC<NodeDetailPanelProps> = ({ node, containerRelations }) => (
  <div>
    <div style={{ fontSize: 12.5, fontWeight: 700, color: C_TEXT, marginBottom: 8 }}>Node detail</div>
    {!node && <div style={{ fontSize: 12, color: C_MUTED }}>Select a node to inspect it.</div>}
    {node && <NodeBody key={node.id} node={node} relations={containerRelations[node.sourceContainerId]} />}
  </div>
);

export default NodeDetailPanel;
