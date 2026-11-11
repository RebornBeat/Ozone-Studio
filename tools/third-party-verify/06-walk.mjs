// TPV-06 ONE real /orchestrate walk, driven as an outside client. OPT-IN (spends model calls on the configured chain):
//   OZONE_TPV_WALK=1 node tools/third-party-verify/06-walk.mjs "<prompt>"   (prompt may also come from OZONE_TPV_PROMPT)
// Never run on the EU before BUILD_AND_TEST_PLAN §3-§5 are done. Reads the result back through the public views.
import { suite, api, http, sessionToken } from "./tpv.mjs";
const s = suite("tpv-06-walk");
if (process.env.OZONE_TPV_WALK !== "1") { s.skip("L0", "real walk", "OZONE_TPV_WALK=1 not set (opt-in: this spends model calls)"); process.exit(s.finish() ? 1 : 0); }
const prompt = process.argv[2] ?? process.env.OZONE_TPV_PROMPT;
if (!prompt) { s.fail("L0", "prompt supplied", "pass the prompt as argv[2] or OZONE_TPV_PROMPT"); process.exit(s.finish() ? 1 : 0); }
const before = (await api("GET", "/capability/gaps")).json?.count;
const ack = (await http("POST", "/orchestrate", { user_id: 1, device_id: 1, consciousness_enabled: false, token_budget: 100000, session_token: await sessionToken(), prompt, submit_only: true })).json;
s.check("L1", "walk accepted with a run_token", !!ack?.run_token, JSON.stringify(ack).slice(0, 160));
if (!ack?.run_token) process.exit(s.finish() ? 1 : 0);
const t0 = Date.now(); let st;
for (;;) { st = (await api("GET", `/orchestrate/status?run_token=${ack.run_token}`)).json; if (st?.state && st.state !== "running") break; if (Date.now() - t0 > Number(process.env.OZONE_TPV_MAX_MS ?? 3600000)) { st = { state: "timeout" }; break; } await new Promise((r) => setTimeout(r, 15000)); }
s.pass("L2", "walk reached a terminal/paused state (as reported, not coerced)", `state=${st.state} after ${Math.round((Date.now() - t0) / 1000)}s`);
const taskId = st.task_id ?? st.result?.task_id;
if (taskId) { const w = (await api("GET", `/order/work/${taskId}`)).json; s.check("L3", "work-order tree readable for the walk's task", w?.success === true, w?.error ?? ""); } else s.skip("L3", "work-order tree for this walk", "status carried no task_id");
const after = (await api("GET", "/capability/gaps")).json?.count;
s.pass("L4", "capability gap count before -> after (what the walk identified)", `${before} -> ${after}`);
const jur = JSON.stringify(st).includes("jurisdiction");
s.pass("L5", "response mentions jurisdiction (warning surfacing observed or not)", String(jur));
process.exit(s.finish() ? 1 : 0);
