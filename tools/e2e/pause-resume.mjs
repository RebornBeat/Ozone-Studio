// Failure doctrine E2E (SPAN_OUT_AND_RESUME §4): a failed run PAUSES (never continues), is visible in
// the Order, is resumable from its checkpoint. Needs a CHAOS host whose model traffic is blocked:
//   target/e2e-chaos (port 50062, HTTPS_PROXY=http://127.0.0.1:9)  -> run with OZONE_HOST=http://127.0.0.1:50062
import fs from "node:fs";
import { suite, http, orchestrate } from "./lib.mjs";

const s = suite("e2e-pause-resume");
const DATA = process.env.OZONE_CHAOS_DATA ?? new URL("../../target/e2e-chaos/zsei_data", import.meta.url).pathname;

const r = await orchestrate({ prompt: "E2E chaos: list two fruits.", consciousness_enabled: false }, { pollMs: 10000, maxMs: 1800000 });
const err = String(r.error ?? "");
s.check("PR1", "run with every model blocked ends PAUSED, not lost", r.state === "failed" && err.includes("PAUSED:"), `state=${r.state} error=${err.slice(0, 160)}`);
const m = err.match(/task (\d+) paused at step (\d+)/);
const taskId = m ? Number(m[1]) : null;
s.check("PR2", "the notice names the task and the step", !!taskId, m ? m[0] : "none");

if (taskId) {
  const info = (await http("POST", "/task/pause_info", { task_id: taskId })).json;
  s.check("PR3", "pause_info: paused_for_failure + resumable (checkpoint on disk)", info.paused_for_failure === true && info.resumable === true, JSON.stringify(info).slice(0, 200));
  const order = (await http("GET", "/order/global")).json;
  const row = (order.paused ?? []).find((t) => t.task_id === taskId);
  s.check("PR4", "Order shows it in the paused group with the error and paused_for_failure", !!row && row.paused_for_failure === true && String(row.error ?? "").includes("paused at step"), JSON.stringify(row ?? {}).slice(0, 200));
  const cp = `${DATA}/runs/${taskId}.json`;
  const cpOk = fs.existsSync(cp);
  let cpJson = {}; try { cpJson = JSON.parse(fs.readFileSync(cp, "utf8")); } catch {}
  s.check("PR5", "checkpoint holds the original request + pause error", cpOk && String(cpJson.request?.prompt).includes("E2E chaos") && String(cpJson.pause_error ?? "").length > 0, cp);
  const again = (await http("POST", "/task/resume_run", { task_id: 999999999 })).json;
  s.check("PR6", "resume of a non-paused/unknown task is refused honestly", again.success === false, JSON.stringify(again).slice(0, 160));
  // the chat turn must have been persisted even though the run failed
  const turns = (await http("GET", "/chat/turns")).json;
  const last = (turns.turns ?? []).slice(-1)[0] ?? {};
  s.check("PR7", "the failed run's prompt is persisted as a ChatTurn (full text)", String(last.prompt).includes("E2E chaos") && last.success === false, `count=${turns.count}`);
  console.log(`\nTASK_ID=${taskId}`);
  fs.writeFileSync("/tmp/e2e_paused_task.txt", String(taskId));
}
process.exit(s.finish() ? 1 : 0);
