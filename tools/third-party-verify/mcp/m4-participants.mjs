// MCP-4 participants: MCP agents are IN the walk. Two parts.
//
//  Part A (no model calls; needs a running host): the AGENT PROVIDER path. Agent A offers itself as a provider; a provider
//  step's call (made here through the host's own POST /mcp/call, exactly the door the walk uses) creates a durable request
//  and AWAITS; the request is visible only to its target; a wrong agent cannot report it; the right agent's report completes
//  it; completed is final (a second report is refused).
//
//  Part B (OPT-IN, spends model calls): OZONE_TPV_WALK=1 node m4-participants.mjs "<prompt>"
//  Agent A starts a walk through ozone_orchestrate naming B and C as participants; B and C discover it with ozone_walk_mine,
//  read the SAME live feed with ozone_walk_feed, and see identical rows (one shared record), each stage row carrying the
//  walk's trace id; the walk is bound to its task id when the task exists.
//
// Everything is read as returned; nothing is inferred. Part A registers one tool row named agent_claude-code (the offer).
import { threeAgents, suite, until, sleep } from "./harness.mjs";
import { api } from "../tpv.mjs";
const s = suite("m4-participants");
const { agents, close } = await threeAgents();
const [a, b, c] = [agents["claude-code"], agents["zcode"], agents["third-agent"]];
try {
  // ── Part A: agent provider ────────────────────────────────────────────────────────────────────────────
  const offer = await a.ozone.call("ozone_agent_offer", { capabilities: ["review a code change and report findings"] });
  s.check("A1", "agent offers itself as a provider (tool row, transport agent, endpoint agent:claude-code)", offer.ok && offer.json?.provider === "agent:claude-code", offer.text.slice(0, 160));
  const tool = offer.json?.name ?? "agent_claude-code";
  const taskId = 900000000 + Math.floor(Math.random() * 90000000);
  const call = { tool, agent: "orchestrator-step", input: { query: "review this change (mcp-4)" }, context: { source: "orchestrator-step", internal: true, task_id: taskId, step_index: 0, site: "provider-step" } };
  const pending = api("POST", "/mcp/call", call); // does NOT resolve until the agent reports
  let early = null; pending.then((r) => { early = r; });
  const req = await until(async () => { const r = (await a.ozone.call("ozone_agent_requests")).json?.requests ?? []; return r.find((x) => x.context?.task_id === taskId) ?? null; }, { everyMs: 500, maxMs: 30000 });
  s.check("A2", "the step's call created a request visible to its target agent", !!req, req ? `id=${req.id} target=${req.target}` : "no request appeared (does the host run this build?)");
  if (!req) process.exit(s.finish() ? 1 : 0);
  s.check("A3", "the call is still waiting: no fail-open, no delegate-and-move-on", early === null, early ? JSON.stringify(early.json).slice(0, 120) : "");
  const bView = (await b.ozone.call("ozone_agent_requests")).json?.requests ?? [];
  s.check("A4", "another agent does not see a request addressed to someone else", !bView.some((x) => x.id === req.id), `${bView.length} pending for zcode`);
  const wrong = await b.ozone.call("ozone_agent_report", { id: req.id, result: { success: true, output: "not mine" } });
  s.check("A5", "a non-target agent's report is refused", !wrong.ok && /may not report/.test(wrong.text), wrong.text.slice(0, 160));
  await sleep(500);
  s.check("A6", "still waiting after the refused report", early === null, "");
  const good = await a.ozone.call("ozone_agent_report", { id: req.id, result: { success: true, output: "findings: none (mcp-4 scripted answer)" } });
  s.check("A7", "the target agent's report is accepted", good.ok && good.json?.success === true, good.text.slice(0, 160));
  const done = await until(() => early, { everyMs: 300, maxMs: 15000 });
  s.check("A8", "the awaiting call completed with exactly the reported result", !!done && done.json?.success === true && JSON.stringify(done.json).includes("findings: none"), done ? JSON.stringify(done.json).slice(0, 200) : "never resolved");
  const again = await a.ozone.call("ozone_agent_report", { id: req.id, result: { success: true, output: "second" } });
  s.check("A9", "completed is final: a second report is refused", !again.ok && /already answered/.test(again.text), again.text.slice(0, 140));

  // ── Part B: a walk with participants (opt-in) ─────────────────────────────────────────────────────────────
  if (process.env.OZONE_TPV_WALK !== "1") { s.skip("B0", "walk with participants", "OZONE_TPV_WALK=1 not set (opt-in: spends model calls)"); process.exit(s.finish() ? 1 : 0); }
  const prompt = process.argv[2] ?? process.env.OZONE_TPV_PROMPT;
  if (!prompt) { s.fail("B0", "prompt supplied", "pass argv[2] or OZONE_TPV_PROMPT"); process.exit(s.finish() ? 1 : 0); }
  const num = (v) => (v === undefined || v === "" ? undefined : Number(v));
  const start = await a.ozone.call("ozone_orchestrate", { prompt, participants: ["zcode", "third-agent"], workspace_id: num(process.env.OZONE_WORKSPACE_ID), project_id: num(process.env.OZONE_PROJECT_ID) });
  const tok = start.json?.run_token;
  s.check("B1", "walk started by an MCP call, participants recorded with the caller first", !!tok && JSON.stringify(start.json?.participants) === JSON.stringify(["claude-code", "zcode", "third-agent"]), start.text.slice(0, 200));
  if (!tok) process.exit(s.finish() ? 1 : 0);
  const mineB = (await b.ozone.call("ozone_walk_mine")).json?.walks ?? [];
  const mineC = (await c.ozone.call("ozone_walk_mine")).json?.walks ?? [];
  s.check("B2", "each named participant discovers the walk it is part of", mineB.some((w) => w.run_token === tok) && mineC.some((w) => w.run_token === tok), `zcode ${mineB.length}, third-agent ${mineC.length}`);
  const fin = await until(async () => { const f = await b.ozone.call("ozone_walk_feed", { run_token: tok }); return f.json?.final ? f.json : null; });
  s.check("B3", "participant B followed the walk to a non-running state through the feed alone", !!fin, fin ? `state=${fin.state}` : "timed out (OZONE_TPV_MAX_MS)");
  const fb = (await b.ozone.call("ozone_walk_feed", { run_token: tok })).json;
  const fc = (await c.ozone.call("ozone_walk_feed", { run_token: tok })).json;
  s.check("B4", "B and C read one shared record: identical rows", JSON.stringify(fb?.rows) === JSON.stringify(fc?.rows), `${fb?.rows?.length} vs ${fc?.rows?.length}`);
  const stages = (fb?.rows ?? []).filter((r) => r.kind === "stage");
  s.check("B5", "stage rows carry the walk's own trace id (set at submission)", stages.length > 0 && stages.every((r) => r.trace_id === fb?.walk?.trace_id), `${stages.length} stage rows`);
  s.check("B6", "the walk is bound to its task id once the task exists", !!fb?.task_id, `task_id=${fb?.task_id}`);
  const items = (fb?.rows ?? []).filter((r) => r.kind === "work_item");
  s.pass("B7", "work-order item changes seen by participants", `${items.length} changes`);
} finally { close(); }
process.exit(s.finish() ? 1 : 0);
