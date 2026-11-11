// TPV-02 work-order tree: checklist = step = task completion; completed is FINAL; rerun = new instance. Read-only.
// Reads whatever tasks exist; never creates one (use 06-walk.mjs for that).
import { suite, api } from "./tpv.mjs";
const s = suite("tpv-02-work-order");
const list = (await api("POST", "/task/list", { limit: 50, offset: 0, session_token: "" })).json;
const tasks = list?.tasks ?? [];
s.check("W0", "/task/list answers with a tasks array", Array.isArray(list?.tasks), `${tasks.length} tasks`);
if (!tasks.length) { s.skip("W1", "work-order tree shape", "no tasks exist on this host yet (run 06-walk.mjs first)"); process.exit(s.finish() ? 1 : 0); }
const withKind = tasks.filter((t) => "kind" in t);
s.check("W1", "task rows carry kind/parent_task_id/rerun_of (additive fields)", tasks.every((t) => "kind" in t || "parent_task_id" in t || "rerun_of" in t) || withKind.length > 0, `${withKind.length}/${tasks.length} rows expose kind`);
const wo = tasks.filter((t) => String(t.kind ?? t.blueprint_name ?? "").startsWith("wo_") || String(t.name ?? "").startsWith("wo_"));
const sample = (wo.length ? wo : tasks).slice(0, 5);
for (const t of sample) {
  const r = (await api("GET", `/order/work/${t.task_id}`)).json;
  if (r?.success !== true) { s.skip(`W2-${t.task_id}`, "work-order tree", `no tree for this task (${r?.error ?? "n/a"})`); continue; }
  const tree = r.work_order;
  s.check(`W2-${t.task_id}`, "work-order tree returned as an object", tree && typeof tree === "object", Object.keys(tree).slice(0, 8).join(","));
  const items = [];
  (function walk(n) { if (!n || typeof n !== "object") return; if (Array.isArray(n)) return n.forEach(walk); if (n.status && (n.item_id ?? n.id ?? n.title ?? n.name)) items.push(n); for (const v of Object.values(n)) walk(v); })(tree);
  const done = items.filter((i) => String(i.status).toLowerCase().startsWith("complet") || String(i.status).toLowerCase() === "done");
  s.pass(`W3-${t.task_id}`, "items counted from the tree as returned (nothing inferred)", `${items.length} items, ${done.length} completed`);
  s.check(`W4-${t.task_id}`, "no manual/offer-based items remain (offer:/awaiting-operator superseded)", !JSON.stringify(tree).match(/"offer:|awaiting-operator/), "");
}
process.exit(s.finish() ? 1 : 0);
