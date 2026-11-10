/**
 * WorkOrderTree — the universal checklist of one task (docs/UNIVERSAL_ORDER_GUIDE.md; operator 2026-11-10).
 *
 * Reads GET /order/work/:task_id — a tree DERIVED at read time from the real task records (never a copy):
 *   walk task → AMT branch (wo_branch, nested like the AMT) → blueprint step (wo_step) and, for every capability gap the walk
 *   reported, a capability task (wo_capability) with its own checklist (judge / methodology / tool / register / replan / verify).
 * The one rule the host enforces and this view only DISPLAYS: a work-order task is complete exactly when all its checklist items are done and
 * all its children are complete. "complete" below is shown ONLY when the host says `complete: true` — nothing is inferred here. Completed is
 * final: a finished item is never shown reopened; a rerun is a NEW task (shown as "rerun of #id"), and an item that is still open is simply
 * "not yet completed".
 *
 * The capability gap's `tool` item has no automatic mechanism (nothing creates or acquires the provider): while it is open it is shown as
 * AWAITING THE OPERATOR'S DECISION (marketplace download after review, or a custom pipeline).
 *
 * REAL DATA ONLY: counts are the host's items_done / items_total (recursed here by plain addition), no percentages, no scores.
 */
import React, { useEffect, useState } from "react";
import { fetchWorkOrder, type WorkOrderItem, type WorkOrderNode } from "../ozoneClient";

const KIND_LABEL: Record<string, string> = {
  wo_branch: "AMT branch",
  wo_step: "Step",
  wo_capability: "Capability gap",
};

function kindLabel(kind: string): string {
  if (KIND_LABEL[kind]) return KIND_LABEL[kind];
  return kind.startsWith("wo_") ? kind.slice(3) : "Task";
}

function isWorkOrderKind(kind: string): boolean {
  return kind.startsWith("wo_");
}

function titleOf(n: WorkOrderNode): string {
  if (typeof n.title === "string" && n.title.trim() !== "") return n.title;
  if (typeof n.key === "string" && n.key !== "") return n.key;
  return `task #${n.task_id}`;
}

function cleanStatus(raw: string): string {
  return String(raw ?? "").replace(/^"+|"+$/g, "");
}

/** Sum the real item counts of a node and everything beneath it. */
function totals(n: WorkOrderNode): { done: number; total: number } {
  let done = Number(n.items_done) || 0;
  let total = Number(n.items_total) || 0;
  for (const c of n.children ?? []) {
    const t = totals(c);
    done += t.done;
    total += t.total;
  }
  return { done, total };
}

function when(ts?: number | null): string {
  if (typeof ts !== "number" || !isFinite(ts) || ts <= 0) return "";
  return new Date(ts * 1000).toLocaleString([], { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
}

const ItemRow: React.FC<{ item: WorkOrderItem }> = ({ item }) => {
  const awaitingOperator = item.key === "tool" && !item.done;
  const cls = item.done ? "owo-item owo-done" : awaitingOperator ? "owo-item owo-await" : "owo-item owo-open";
  return (
    <li className={cls}>
      <span className="owo-box" aria-hidden="true">{item.done ? "☑" : "☐"}</span>
      <span className="owo-item-text">
        <span className="owo-state">{item.done ? "done" : awaitingOperator ? "awaiting operator decision" : "not yet completed"}</span>{" "}
        {item.text}
        {item.done && (item.evidence || item.completed_at || item.by) && (
          <span className="owo-evidence">
            {" "}— {[item.evidence, item.by ? `by ${item.by}` : "", when(item.completed_at)].filter((x) => x && String(x).trim() !== "").join(" · ")}
          </span>
        )}
        {item.ref ? <span className="owo-evidence"> · ref {item.ref}</span> : null}
        {awaitingOperator && (
          <span className="owo-evidence"> — nothing creates or acquires this tool automatically: marketplace download after review, or a custom pipeline</span>
        )}
      </span>
    </li>
  );
};

const NodeView: React.FC<{ node: WorkOrderNode; depth: number; onOpenTask?: (id: number) => void }> = ({ node, depth, onOpenTask }) => {
  const wo = isWorkOrderKind(node.kind);
  const t = totals(node);
  const status = cleanStatus(node.status);
  // "complete" only when the host derived it for a work-order task; the walk's own task shows ITS OWN lifecycle status
  const state = wo ? (node.complete ? "complete" : "open") : status || "task";
  const cls = `owo-node ${wo ? (node.complete ? "owo-complete" : "owo-pending") : "owo-root"}`;
  const checklist: WorkOrderItem[] = Array.isArray(node.checklist) ? node.checklist : [];
  const children: WorkOrderNode[] = Array.isArray(node.children) ? node.children : [];
  return (
    <details className={cls} open={!(wo && node.complete) && depth < 3}>
      <summary>
        <span className="owo-kind">{kindLabel(node.kind)}</span>
        <span className="owo-title">{titleOf(node)}</span>
        <span className={`owo-badge owo-badge-${wo ? (node.complete ? "ok" : "open") : "plain"}`}>{state}</span>
        {t.total > 0 && <span className="owo-count">{t.done}/{t.total} items</span>}
        {typeof node.rerun_of === "number" && node.rerun_of > 0 && (
          <button type="button" className="owo-link" onClick={(e) => { e.preventDefault(); onOpenTask?.(node.rerun_of as number); }} title="this is a NEW task instance that re-runs the earlier one; the earlier task's record is unchanged">
            rerun of #{node.rerun_of}
          </button>
        )}
        {wo && node.complete && node.completed_at ? <span className="owo-when">completed {when(node.completed_at)}</span> : null}
      </summary>
      {checklist.length > 0 && (
        <ul className="owo-items">
          {checklist.map((it) => <ItemRow key={`${node.task_id}:${it.key}`} item={it} />)}
        </ul>
      )}
      {children.length > 0 && (
        <div className="owo-children">
          {children.map((c) => <NodeView key={c.task_id} node={c} depth={depth + 1} onOpenTask={onOpenTask} />)}
        </div>
      )}
    </details>
  );
};

export interface WorkOrderTreeProps {
  taskId: number;
  /** change this (e.g. the task's status + step count) to refetch immediately; the tree also re-polls on its own */
  refreshKey?: string;
  /** open another task (a rerun's origin) in the task view */
  onOpenTask?: (id: number) => void;
}

export const WorkOrderTree: React.FC<WorkOrderTreeProps> = ({ taskId, refreshKey, onOpenTask }) => {
  const [tree, setTree] = useState<WorkOrderNode | null>(null);
  const [missing, setMissing] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    let cancelled = false;
    const load = async (): Promise<void> => {
      try {
        const r = await fetchWorkOrder(taskId);
        if (cancelled) return;
        if (r && r.success && r.work_order) {
          setTree(r.work_order);
          setMissing(false);
          setError("");
        } else {
          setTree(null);
          setMissing(true);
        }
      } catch (e: unknown) {
        if (!cancelled) setError(e instanceof Error ? e.message : "could not load the work order");
      }
    };
    void load();
    const id = window.setInterval(() => { void load(); }, 8000);
    return () => { cancelled = true; window.clearInterval(id); };
  }, [taskId, refreshKey]);

  if (error) return <div className="oempty" style={{ textAlign: "left" }}>Work order unavailable: {error}</div>;
  if (missing || !tree) return null;
  const hasStructure = (tree.children?.length ?? 0) > 0 || (tree.checklist?.length ?? 0) > 0;
  const t = totals(tree);
  return (
    <div className="owo" style={{ marginTop: 12 }}>
      <div className="omono" style={{ color: "#e8eef6", fontWeight: 600, marginBottom: 6 }}>
        Work order{t.total > 0 ? <span className="owo-count"> {t.done}/{t.total} items done</span> : null}
      </div>
      {hasStructure
        ? <NodeView node={tree} depth={0} onOpenTask={onOpenTask} />
        : <div className="oempty" style={{ textAlign: "left" }}>No work order recorded for this task (it ran before work orders existed, or it has not reached blueprint design).</div>}
    </div>
  );
};

export default WorkOrderTree;
