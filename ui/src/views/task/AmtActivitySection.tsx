/**
 * H4 — AMT activity summary in task detail.
 *
 * `amt_summary` is already returned inline by `POST /task/get` (same endpoint
 * TaskDetailPanel.fetchTask uses) — confirmed live (task 25):
 *   { branch_count: number, branches: [{ id, content, depth, confidence,
 *     verified, relationships: [] }] }
 * `TaskInfo` in TaskDetailPanel.tsx doesn't type this field yet, so it's read
 * defensively here off `task: any`. Every real task checked live (ids 1-3, 5,
 * 10-24) has `amt_summary: null` — task 25 is the only one seen with real
 * branch data, so the empty case is common and must look intentional, not
 * broken.
 *
 * NOT extended with the D4 fork/main lineage loader (`loadAmtGenerations`),
 * despite that being the original plan for this fork — verified live that the
 * bridge doesn't exist: `/task/get`'s payload carries no `project_id` (or any
 * other project reference) anywhere, and `branches[].id` is a plain integer
 * scoped to this run's in-memory AMT tree (e.g. `11`, `1`, `2`...), not a ZSEI
 * container id — the persisted AMT generations `loadAmtGenerations` walks use
 * much larger, globally-unique container ids (e.g. 30411, 40059). There is no
 * real field connecting a task's `amt_summary` to a specific persisted AMT
 * container, so no lineage cross-reference is attempted. Flagged in the
 * handoff rather than faked.
 */
import React, { useState } from "react";

interface AmtBranchRelationship {
  target_id?: number;
  relation_type?: string;
  confidence?: number;
  [extra: string]: unknown;
}

interface AmtBranch {
  id: number;
  content: string;
  depth: number;
  confidence: number;
  verified: boolean;
  relationships?: AmtBranchRelationship[];
  [extra: string]: unknown;
}

interface AmtSummary {
  branch_count: number;
  branches: AmtBranch[];
}

const C_TEXT = "#dfe7f2";
const C_BODY = "#c7d0dc";
const C_MUTED = "#8b98ab";
const C_BORDER = "#1e2836";

function isAmtSummary(v: unknown): v is AmtSummary {
  return (
    !!v &&
    typeof v === "object" &&
    Array.isArray((v as AmtSummary).branches) &&
    typeof (v as AmtSummary).branch_count === "number"
  );
}

const BranchRow: React.FC<{ branch: AmtBranch }> = ({ branch }) => {
  const [expanded, setExpanded] = useState(false);
  const rels = Array.isArray(branch.relationships) ? branch.relationships : [];
  return (
    <div
      style={{
        marginLeft: Math.min(branch.depth, 8) * 14,
        borderLeft: branch.depth > 0 ? `1px solid ${C_BORDER}` : "none",
        paddingLeft: branch.depth > 0 ? 8 : 0,
        marginBottom: 4,
      }}
    >
      <div
        style={{ display: "flex", gap: 8, alignItems: "baseline", cursor: rels.length > 0 ? "pointer" : "default" }}
        onClick={() => rels.length > 0 && setExpanded((e) => !e)}
      >
        <span style={{ fontSize: 10.5, color: C_MUTED, minWidth: 28 }}>#{branch.id}</span>
        <span
          style={{
            fontSize: 10,
            color: branch.verified ? "#8fe38f" : "#e8c14f",
            border: `1px solid ${branch.verified ? "#8fe38f" : "#e8c14f"}`,
            borderRadius: 4,
            padding: "0 4px",
          }}
          title={branch.verified ? "verified" : "not verified"}
        >
          {branch.verified ? "verified" : "unverified"}
        </span>
        <span style={{ fontSize: 10.5, color: C_MUTED }}>conf {branch.confidence.toFixed(2)}</span>
        <span style={{ fontSize: 12, color: C_BODY }}>{branch.content}</span>
        {rels.length > 0 && (
          <span style={{ fontSize: 10.5, color: C_MUTED }}>
            {expanded ? "▾" : "▸"} {rels.length} relation{rels.length === 1 ? "" : "s"}
          </span>
        )}
      </div>
      {expanded && rels.length > 0 && (
        <div style={{ marginLeft: 36, marginTop: 2 }}>
          {rels.map((r, i) => (
            <div key={i} style={{ fontSize: 10.5, color: C_MUTED }}>
              {r.relation_type ?? "relation"} → #{r.target_id ?? "?"}
              {typeof r.confidence === "number" ? ` (conf ${r.confidence.toFixed(2)})` : ""}
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

export const AmtActivitySection: React.FC<{ task: any }> = ({ task }) => {
  const summary: unknown = task?.amt_summary;

  return (
    <div>
      <div style={{ fontSize: 12.5, fontWeight: 700, color: C_TEXT, marginBottom: 6 }}>AMT activity</div>
      {!isAmtSummary(summary) ? (
        <div style={{ fontSize: 12, color: C_MUTED, fontStyle: "italic" }}>
          No AMT activity recorded for this task — <code>amt_summary</code> is <code>null</code> (the common case;
          verified live that most tasks never populate it).
        </div>
      ) : (
        <>
          <div style={{ fontSize: 11.5, color: C_MUTED, marginBottom: 8 }}>
            {summary.branch_count} branch{summary.branch_count === 1 ? "" : "es"}
          </div>
          <div>
            {summary.branches.map((b) => (
              <BranchRow key={b.id} branch={b} />
            ))}
          </div>
          <div style={{ fontSize: 10.5, color: C_MUTED, marginTop: 10, fontStyle: "italic" }}>
            Fork/main lineage from the persisted AMT container graph isn't shown here — this task's payload carries
            no project id or container id that maps to a real, persisted AMT generation (verified live), only this
            run's own in-memory branch ids.
          </div>
        </>
      )}
    </div>
  );
};

export default AmtActivitySection;
