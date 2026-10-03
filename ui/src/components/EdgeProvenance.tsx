/**
 * C13 — Edge confidence/provenance display (Graph View sidebar).
 *
 * GraphView passes the hovered edge, else the selected edge, else null.
 *
 * What provenance edges REALLY carry (confirmed against the pipeline sources in
 * assets/pipelines/modalities/{code,math,text}/main.rs and real persisted graph
 * files — not the plan's assumption, which was made about container-level
 * `Relation`s, not in-graph edges):
 *
 *   code  — CodeGraphEdge{from_node,to_node,edge_type,weight,properties}. NO
 *           provenance field. `weight` is the literal 1.0 at every construction
 *           site (a constant, not a measurement). `properties` is empty except
 *           Calls edges: {line, is_method}.
 *   math  — MathGraphEdge{edge_id,...same}. NO provenance field. weight is always
 *           1.0, properties always empty.
 *   text  — TextGraphEdge additionally has a persisted `provenance` enum
 *           (EdgeProvenance), created_by_step?, version, version_notes,
 *           is_cross_modal, cross_modal_index_id?, grammar_info?. `weight` is a
 *           real score only for Document->Entity (entity confidence) and
 *           Document->Topic/Keyword (relevance) Contains edges; 1.0 elsewhere.
 *           `properties` carries per-edge evidence for grammar / cross-sentence /
 *           coreference edges (verb/tense/negated, evidence, canonical_form, ...).
 *           Caveat shown in the UI: text_new_edge — the only TextGraphEdge
 *           constructor — stamps provenance = DerivedFromPrompt on every edge.
 *
 * Container-level relations (`ContainerRelation`, produced later by C8-C10) do
 * carry confidence + discovered_via (+ graph_hops); those are rendered verbatim.
 * `discovered_via` is shown as whatever real string is present — there is no
 * "MathAnalysis" variant in the real DiscoveryMethod enum, so none is ever
 * synthesized. Anything absent is omitted or stated as absent, never defaulted.
 */
import React from "react";
import type { ContainerRelation, GraphViewEdge, Modality } from "../graphViewTypes";

export interface EdgeProvenanceProps {
  edge: GraphViewEdge | null;
}

const C_TEXT = "var(--color-text)";
const C_BODY = "var(--color-text-secondary)";
const C_MUTED = "var(--color-text-muted)";
const C_BORDER = "var(--color-border-faint)";
const C_WARN = "#e8c14f";

const wrap: React.CSSProperties = { overflowWrap: "anywhere", wordBreak: "break-word", minWidth: 0 };

const MODALITIES: readonly string[] = ["code", "math", "text"];

/** graphViewData.ts builds edge ids as `${modality}:${containerId}:...`. */
function modalityOfEdge(edge: GraphViewEdge): Modality | null {
  const prefix = edge.id.split(":")[0];
  return MODALITIES.includes(prefix) ? (prefix as Modality) : null;
}

function isContainerRelation(raw: unknown): raw is ContainerRelation {
  if (!raw || typeof raw !== "object") return false;
  const o = raw as Record<string, unknown>;
  return "target_id" in o && "relation_type" in o && !("from_node" in o);
}

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

/** Rust enum serialization: unit variants are strings, tuple variants are {Variant: payload}. */
function formatEnumish(v: unknown): string {
  if (v && typeof v === "object" && !Array.isArray(v)) {
    const entries = Object.entries(v as Record<string, unknown>);
    if (entries.length === 1) return `${entries[0][0]}(${formatScalar(entries[0][1])})`;
  }
  return formatScalar(v);
}

const SectionTitle: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <div
    style={{
      fontSize: 11,
      fontWeight: 700,
      letterSpacing: 0.4,
      textTransform: "uppercase",
      color: C_MUTED,
      margin: "10px 0 4px",
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

const Note: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <div style={{ ...wrap, fontSize: 11, color: C_MUTED, margin: "2px 0 4px", lineHeight: 1.45 }}>{children}</div>
);

const Empty: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <div style={{ ...wrap, fontSize: 11.5, color: C_MUTED, fontStyle: "italic" }}>{children}</div>
);

const Card: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <div style={{ marginTop: 16, paddingTop: 12, borderTop: `1px solid ${C_BORDER}` }}>
    <div style={{ fontSize: 12.5, fontWeight: 700, color: C_TEXT, marginBottom: 6 }}>Edge provenance</div>
    <div style={{ fontSize: 12, color: C_BODY, lineHeight: 1.6 }}>{children}</div>
  </div>
);

function weightNote(modality: Modality | null, weight: number): string | null {
  if (modality === "code" || modality === "math") {
    return weight === 1
      ? "Constant 1.0 — every " + modality + " edge is built with a hardcoded weight, so this is not a measured score."
      : null;
  }
  if (modality === "text") {
    return "A real score only on Document→Entity (entity confidence) and Document→Topic/Keyword (relevance) Contains edges; every other text edge is 1.0.";
  }
  return null;
}

const ContainerRelationBody: React.FC<{ edge: GraphViewEdge; rel: ContainerRelation }> = ({ edge, rel }) => {
  const r = rel as unknown as Record<string, unknown>;
  return (
    <>
      <Row label="Relation">{formatScalar(rel.relation_type)}</Row>
      <Row label="Class">{edge.edgeClass}</Row>
      <Row label="Target container">{formatScalar(rel.target_id)}</Row>

      <SectionTitle>Recorded on the relation</SectionTitle>
      {typeof rel.confidence === "number" ? (
        <Row label="Confidence">{rel.confidence.toFixed(2)}</Row>
      ) : (
        <Empty>No confidence recorded on this relation.</Empty>
      )}
      {isPresent(rel.discovered_via) ? (
        <Row label="Discovered via">{formatScalar(rel.discovered_via)}</Row>
      ) : (
        <Empty>No discovery method recorded on this relation.</Empty>
      )}
      {isPresent(r.graph_hops) && <Row label="Graph hops">{formatScalar(r.graph_hops)}</Row>}
    </>
  );
};

const InGraphBody: React.FC<{ edge: GraphViewEdge }> = ({ edge }) => {
  const raw = edge.raw as unknown as Record<string, unknown>;
  const modality = modalityOfEdge(edge);
  const weight = typeof raw.weight === "number" ? raw.weight : null;
  const properties =
    raw.properties && typeof raw.properties === "object" && !Array.isArray(raw.properties)
      ? (raw.properties as Record<string, unknown>)
      : {};
  const propEntries = Object.entries(properties);

  const grammar =
    raw.grammar_info && typeof raw.grammar_info === "object" ? (raw.grammar_info as Record<string, unknown>) : null;

  // Provenance = fields that say where an edge came from. Plain attributes
  // (weight, properties like Calls' line/is_method) are shown separately and do
  // NOT count as provenance, so code/math edges honestly report none.
  const provenanceRows: React.ReactNode[] = [];
  if (isPresent(raw.provenance)) {
    const value = formatEnumish(raw.provenance);
    provenanceRows.push(
      <React.Fragment key="provenance">
        <Row label="Source object">{value}</Row>
        {value === "DerivedFromPrompt" && (
          <Note>
            Set unconditionally by the text pipeline's edge constructor (text_new_edge) on every edge it builds — it
            names the source-object kind, not a per-edge measurement.
          </Note>
        )}
      </React.Fragment>,
    );
  }
  if (typeof raw.created_by_step === "number") {
    provenanceRows.push(<Row key="step" label="Created by step">{raw.created_by_step}</Row>);
  }
  if (typeof raw.version === "number") {
    provenanceRows.push(<Row key="version" label="Version">{raw.version}</Row>);
  }
  if (Array.isArray(raw.version_notes) && raw.version_notes.length > 0) {
    provenanceRows.push(<Row key="vnotes" label="Version notes">{raw.version_notes.length}</Row>);
  }
  if (raw.is_cross_modal === true) {
    provenanceRows.push(
      <Row key="xm" label="Cross-modal">
        yes{isPresent(raw.cross_modal_index_id) ? ` (index ${formatScalar(raw.cross_modal_index_id)})` : ""}
      </Row>,
    );
  }
  if (grammar) {
    provenanceRows.push(
      <div key="grammar" style={{ marginTop: 2 }}>
        <div style={{ color: C_MUTED }}>Grammar-derived:</div>
        {Object.entries(grammar).map(([k, v]) =>
          isPresent(v) ? (
            <div key={k} style={{ ...wrap, marginLeft: 8, color: C_BODY }}>
              <span style={{ color: C_MUTED }}>{k}: </span>
              {formatEnumish(v)}
            </div>
          ) : null,
        )}
      </div>,
    );
  }
  if (isPresent(edge.discoveredVia)) {
    provenanceRows.push(<Row key="dv" label="Discovered via">{formatScalar(edge.discoveredVia)}</Row>);
  }

  const wNote = weight !== null ? weightNote(modality, weight) : null;

  return (
    <>
      <Row label="Type">{edge.edgeType}</Row>
      <Row label="Class">{edge.edgeClass}</Row>
      {typeof raw.from_node === "number" && typeof raw.to_node === "number" && (
        <Row label="Endpoints">
          {raw.from_node} → {raw.to_node}
        </Row>
      )}
      {!edge.isReal && (
        <div
          style={{
            ...wrap,
            border: `1px solid ${C_WARN}`,
            color: C_WARN,
            borderRadius: 6,
            padding: "6px 8px",
            margin: "6px 0",
            fontSize: 11.5,
          }}
        >
          Schema-only type — no construction site produces this edge type in the current pipelines.
        </div>
      )}

      <SectionTitle>Weight / confidence</SectionTitle>
      {weight !== null ? (
        <>
          <Row label="Weight">{weight}</Row>
          {wNote && <Note>{wNote}</Note>}
        </>
      ) : (
        <Empty>No weight recorded on this edge.</Empty>
      )}
      {typeof edge.confidence === "number" && <Row label="Confidence">{edge.confidence.toFixed(2)}</Row>}

      <SectionTitle>Provenance</SectionTitle>
      {provenanceRows.length > 0 ? provenanceRows : <Empty>No provenance recorded on this edge</Empty>}

      {propEntries.length > 0 && (
        <>
          <SectionTitle>Recorded properties</SectionTitle>
          {propEntries.map(([k, v]) => (
            <Row key={k} label={k}>
              {formatScalar(v)}
            </Row>
          ))}
        </>
      )}
    </>
  );
};

export const EdgeProvenance: React.FC<EdgeProvenanceProps> = ({ edge }) => {
  if (!edge) return null;
  return (
    <Card>
      {isContainerRelation(edge.raw) ? (
        <ContainerRelationBody edge={edge} rel={edge.raw} />
      ) : (
        <InGraphBody edge={edge} />
      )}
    </Card>
  );
};

export default EdgeProvenance;
