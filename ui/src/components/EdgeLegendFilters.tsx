/**
 * C12 — Edge legend + filter controls (Graph View sidebar).
 *
 * Everything shown is derived from the edges actually loaded (`props.edges`,
 * unfiltered): classes and types that don't occur are not listed, and there
 * are no placeholder/"coming soon" categories. Swatches come from the same
 * `edgeVisual()` the canvas uses, via one representative edge per type, so the
 * legend can't drift from what is drawn.
 *
 * GraphView owns the filter state. An edge is hidden when its edgeClass is in
 * hiddenEdgeClasses OR its edgeType is in hiddenEdgeTypes — this component
 * only renders controls and reports toggles. Note hiddenEdgeTypes is keyed by
 * edgeType string alone (not per modality), so toggling e.g. "Contains" hides
 * it in every modality; the modality tags on each row make that visible.
 */
import React, { useMemo } from "react";
import type { EdgeClass, GraphViewEdge, Modality } from "../graphViewTypes";
import { edgeVisual } from "../graphRenderers";

export interface EdgeLegendFiltersProps {
  /** ALL edges currently loaded (unfiltered) — derive the legend from what's
   * actually present, never a hardcoded/aspirational list. */
  edges: GraphViewEdge[];
  hiddenEdgeClasses: Set<EdgeClass>;
  hiddenEdgeTypes: Set<string>;
  onToggleClass: (c: EdgeClass) => void;
  onToggleType: (edgeType: string) => void;
}

// Display order only — a class is rendered solely if edges of that class exist.
const CLASS_ORDER: EdgeClass[] = [
  "structural",
  "dependency",
  "semantic",
  "cross-modal",
  "lineage",
  "governance",
];

const CLASS_HINT: Record<EdgeClass, string> = {
  structural: "Same-graph structure (Contains, PartOf, ...)",
  dependency: "Code dependency edges (Imports, Calls, Extends, Implements)",
  semantic: "Content-derived edges within a graph",
  "cross-modal": "Edges linking different modality graphs",
  lineage: "AMT fork / continuation lineage",
  governance: "Jurisdiction relationships",
};

const MODALITIES: readonly string[] = ["code", "math", "text"];

/** graphViewData.ts builds edge ids as `${modality}:${containerId}:...`. */
function modalityOfEdge(edge: GraphViewEdge): Modality | null {
  const prefix = edge.id.split(":")[0];
  return MODALITIES.includes(prefix) ? (prefix as Modality) : null;
}

interface TypeRow {
  edgeType: string;
  count: number;
  notRealCount: number;
  modalities: string[];
  representative: GraphViewEdge;
}

interface ClassGroup {
  edgeClass: EdgeClass;
  count: number;
  types: TypeRow[];
}

const C_TEXT = "#dfe7f2";
const C_BODY = "#c7d0dc";
const C_MUTED = "#8b98ab";
const C_BORDER = "#1e2836";
const C_WARN = "#e8c14f";

const Swatch: React.FC<{ edge: GraphViewEdge; dim: boolean }> = ({ edge, dim }) => {
  const v = edgeVisual(edge, modalityOfEdge(edge));
  return (
    <svg width={32} height={12} style={{ flexShrink: 0, opacity: dim ? 0.35 : 1 }} aria-hidden="true">
      <line
        x1={2}
        y1={6}
        x2={arrowOffset(v.arrow)}
        y2={6}
        stroke={v.stroke}
        strokeWidth={Math.max(1, v.strokeWidth)}
        strokeDasharray={v.strokeDasharray}
        opacity={v.opacity ?? 1}
      />
      {v.arrow && <polygon points="30,6 24,2.5 24,9.5" fill="#6b7a90" />}
    </svg>
  );
};

function arrowOffset(arrow: boolean | undefined): number {
  return arrow ? 24 : 30;
}

export const EdgeLegendFilters: React.FC<EdgeLegendFiltersProps> = ({
  edges,
  hiddenEdgeClasses,
  hiddenEdgeTypes,
  onToggleClass,
  onToggleType,
}) => {
  const groups = useMemo<ClassGroup[]>(() => {
    const byClass = new Map<EdgeClass, Map<string, TypeRow>>();
    for (const edge of edges) {
      let types = byClass.get(edge.edgeClass);
      if (!types) {
        types = new Map();
        byClass.set(edge.edgeClass, types);
      }
      const modality = modalityOfEdge(edge);
      const existing = types.get(edge.edgeType);
      if (existing) {
        existing.count += 1;
        if (!edge.isReal) existing.notRealCount += 1;
        if (modality && !existing.modalities.includes(modality)) existing.modalities.push(modality);
      } else {
        types.set(edge.edgeType, {
          edgeType: edge.edgeType,
          count: 1,
          notRealCount: edge.isReal ? 0 : 1,
          modalities: modality ? [modality] : [],
          representative: edge,
        });
      }
    }
    return CLASS_ORDER.filter((c) => byClass.has(c)).map((c) => {
      const types = Array.from(byClass.get(c)!.values()).sort((a, b) => b.count - a.count || a.edgeType.localeCompare(b.edgeType));
      return { edgeClass: c, count: types.reduce((n, t) => n + t.count, 0), types };
    });
  }, [edges]);

  const visibleCount = useMemo(
    () => edges.filter((e) => !hiddenEdgeClasses.has(e.edgeClass) && !hiddenEdgeTypes.has(e.edgeType)).length,
    [edges, hiddenEdgeClasses, hiddenEdgeTypes],
  );

  return (
    <div style={{ marginTop: 16, paddingTop: 12, borderTop: `1px solid ${C_BORDER}` }}>
      <div style={{ fontSize: 12.5, fontWeight: 700, color: C_TEXT, marginBottom: 6 }}>Edge legend &amp; filters</div>

      {edges.length === 0 ? (
        <div style={{ fontSize: 12, color: C_MUTED }}>No edges loaded — nothing to filter.</div>
      ) : (
        <>
          <div style={{ fontSize: 11, color: C_MUTED, marginBottom: 8 }}>
            {visibleCount} of {edges.length} edges visible
          </div>

          {groups.map((group) => {
            const classHidden = hiddenEdgeClasses.has(group.edgeClass);
            return (
              <div key={group.edgeClass} style={{ marginBottom: 10 }}>
                <label
                  title={CLASS_HINT[group.edgeClass]}
                  style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer", fontSize: 12, fontWeight: 600 }}
                >
                  <input
                    type="checkbox"
                    checked={!classHidden}
                    onChange={() => onToggleClass(group.edgeClass)}
                    style={{ margin: 0 }}
                  />
                  <span
                    style={{
                      color: classHidden ? C_MUTED : C_TEXT,
                      textDecoration: classHidden ? "line-through" : "none",
                    }}
                  >
                    {group.edgeClass}
                  </span>
                  <span style={{ marginLeft: "auto", color: C_MUTED, fontWeight: 400, fontSize: 11 }}>{group.count}</span>
                </label>

                <div style={{ marginLeft: 18, marginTop: 3 }}>
                  {group.types.map((t) => {
                    const typeHidden = hiddenEdgeTypes.has(t.edgeType);
                    const effectivelyHidden = classHidden || typeHidden;
                    const tags = [
                      t.modalities.length > 0 ? t.modalities.join("/") : null,
                      t.notRealCount > 0
                        ? t.notRealCount === t.count
                          ? "schema-only"
                          : `${t.notRealCount} schema-only`
                        : null,
                    ].filter(Boolean);
                    return (
                      <label
                        key={t.edgeType}
                        title={
                          classHidden
                            ? `Hidden because the "${group.edgeClass}" class is hidden`
                            : `Toggle "${t.edgeType}" edges (applies to every modality)`
                        }
                        style={{
                          display: "flex",
                          alignItems: "center",
                          gap: 6,
                          padding: "2px 0",
                          fontSize: 11.5,
                          cursor: classHidden ? "default" : "pointer",
                        }}
                      >
                        <input
                          type="checkbox"
                          checked={!effectivelyHidden}
                          disabled={classHidden}
                          onChange={() => onToggleType(t.edgeType)}
                          style={{ margin: 0 }}
                        />
                        <Swatch edge={t.representative} dim={effectivelyHidden} />
                        <span style={{ display: "flex", flexDirection: "column", minWidth: 0, flex: 1 }}>
                          <span
                            style={{
                              color: effectivelyHidden ? C_MUTED : C_BODY,
                              textDecoration: effectivelyHidden ? "line-through" : "none",
                              overflow: "hidden",
                              textOverflow: "ellipsis",
                              whiteSpace: "nowrap",
                            }}
                          >
                            {t.edgeType}
                          </span>
                          {tags.length > 0 && (
                            <span
                              style={{
                                fontSize: 10,
                                color: t.notRealCount > 0 ? C_WARN : C_MUTED,
                                opacity: effectivelyHidden ? 0.6 : 1,
                              }}
                            >
                              {tags.join(" · ")}
                            </span>
                          )}
                        </span>
                        <span style={{ color: C_MUTED, fontSize: 11 }}>{t.count}</span>
                      </label>
                    );
                  })}
                </div>
              </div>
            );
          })}
        </>
      )}
    </div>
  );
};

export default EdgeLegendFilters;
