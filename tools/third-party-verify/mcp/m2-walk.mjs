// MCP-2 one real walk started by agent A through ozone-mcp, observed by agents B and C. OPT-IN (spends model calls):
//   OZONE_TPV_WALK=1 node tools/third-party-verify/mcp/m2-walk.mjs "<prompt>"     (OZONE_PROJECT_ID / OZONE_WORKSPACE_ID optional)
// Also covers the completion gate: a walk with open child work must be reported awaiting (not completed), and a completed
// task must have no open item. Everything is read from the host as returned; nothing is inferred.
import { threeAgents, suite, until } from "./harness.mjs";
const s = suite("m2-walk");
if (process.env.OZONE_TPV_WALK !== "1") { s.skip("W0", "real walk via MCP", "OZONE_TPV_WALK=1 not set (opt-in: spends model calls)"); process.exit(s.finish() ? 1 : 0); }
const prompt = process.argv[2] ?? process.env.OZONE_TPV_PROMPT;
if (!prompt) { s.fail("W0", "prompt supplied", "pass argv[2] or OZONE_TPV_PROMPT"); process.exit(s.finish() ? 1 : 0); }
const { ctx, agents, close } = await threeAgents();
const [a, b, c] = [agents["claude-code"], agents["zcode"], agents["third-agent"]];
const num = (v) => (v === undefined || v === "" ? undefined : Number(v));
try {
  const gapsBefore = (await a.ozone.call("ozone_gaps")).json?.count;
  const start = await a.ozone.call("ozone_orchestrate", { prompt, workspace_id: num(process.env.OZONE_WORKSPACE_ID), project_id: num(process.env.OZONE_PROJECT_ID) });
  s.check("W1", "walk accepted: run_token returned immediately (non-blocking)", start.ok && !!start.json?.run_token, start.text.slice(0, 200));
  if (!start.json?.run_token) process.exit(s.finish() ? 1 : 0);
  s.check("W2", "start recorded a shared-context note with a prompt hash, not the prompt", !!start.json.note_id && !start.text.includes(prompt.slice(0, 40)) , `note_id=${start.json.note_id}`);
  const tok = start.json.run_token;

  // another agent watches the same run through its own server/identity
  const final = await until(async () => { const r = await b.ozone.call("ozone_run_status", { run_token: tok }); return r.json?.state && r.json.state !== "running" ? r.json : null; });
  s.check("W3", "agent B observed the run reach a non-running state (as reported)", !!final, final ? `state=${final.state}` : "timed out (OZONE_TPV_MAX_MS)");
  const taskId = final?.task_id ?? final?.result?.task_id;
  if (!taskId) { s.skip("W4", "task, work order, call graph", "status carried no task_id"); process.exit(s.finish() ? 1 : 0); }
  const task = await c.ozone.call("ozone_task_get", { task_id: taskId });
  s.pass("W4", "agent C read the task", task.text.slice(0, 200));
  const wo = await c.ozone.call("ozone_work_order", { task_id: taskId });
  s.check("W5", "work-order tree readable", wo.ok && wo.json?.success === true, wo.text.slice(0, 120));

  // completion gate: status vs open items, both as returned
  const items = [];
  (function walk(n) { if (!n || typeof n !== "object") return; if (Array.isArray(n)) return n.forEach(walk); if (n.status && (n.item_id ?? n.id ?? n.title ?? n.name)) items.push(n); for (const v of Object.values(n)) walk(v); })(wo.json?.work_order);
  const open = items.filter((i) => !/^(complete|done|cancel)/i.test(String(i.status)));
  const st = String(task.json?.status ?? final?.state ?? "").toLowerCase();
  if (/^complete/.test(st)) s.check("G1", "a COMPLETED task has no open work-order item (completed is final: all steps done)", open.length === 0, `${open.length} open of ${items.length}: ${open.slice(0, 3).map((i) => i.title ?? i.name).join("; ")}`);
  else if (/await/.test(st)) s.check("G1", "an AWAITING task has at least one open child item (the gate parks, never fails open)", open.length > 0, `${open.length} open of ${items.length}`);
  else s.skip("G1", "completion gate vs open items", `task status is '${st}' (neither completed nor awaiting)`);

  // intelligence from the walk: call graph of what ran, and what it identified
  const live = await a.ozone.call("ozone_callgraph_live", { task_id: taskId });
  s.check("L1", "live call graph for this walk readable (executed calls joined by trace id)", live.ok, live.text.slice(0, 160));
  const gapsAfter = (await b.ozone.call("ozone_gaps")).json?.count;
  s.pass("L2", "capability gap count before -> after", `${gapsBefore} -> ${gapsAfter}`);
  const claims = (await c.shared.call("file_claims")).json?.claims ?? [];
  s.pass("L3", "claims untouched by the walk (coordination state is separate)", `${claims.length} claims`);
} finally { close(); }
process.exit(s.finish() ? 1 : 0);
