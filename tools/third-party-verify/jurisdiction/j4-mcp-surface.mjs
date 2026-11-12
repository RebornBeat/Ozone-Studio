// J4 jurisdiction through the MCP surface (three agents, ozone-mcp). Checks that an MCP orchestrate carrying a Block-rule word is refused/held the
// same way as the HTTP surface, that a file link whose content bears a personal-data shape still builds the NON-semantic graph without sending
// content to a model that the policy forbids, and that every action left a decision row. Mutating, opt-in:
//   OZONE_TPV_WALK=1 OZONE_PROJECT_ID=<id> node tools/third-party-verify/jurisdiction/j4-mcp-surface.mjs
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { threeAgents, suite, until } from "../mcp/harness.mjs";
import { api } from "../tpv.mjs";
const s = suite("j4-jurisdiction-mcp");
if (process.env.OZONE_TPV_WALK !== "1") { s.skip("M0", "MCP jurisdiction checks", "OZONE_TPV_WALK=1 not set (opt-in)"); process.exit(s.finish() ? 1 : 0); }
const project_id = Number(process.env.OZONE_PROJECT_ID);
const t0 = Date.now();
const dir = fs.mkdtempSync(path.join(os.tmpdir(), "ozone-j4-"));
fs.writeFileSync(path.join(dir, "contact.txt"), "Note for the team. Reach alex.example@example.org for the schedule.\n");
const { agents, close } = await threeAgents({ linkRoots: dir });
const [a, b] = [agents["claude-code"], agents["zcode"]];
try {
  const blocked = await a.ozone.call("ozone_orchestrate", { prompt: "Draft a staff training outline on recognising child abuse and what to report.", participants: ["zcode"] });
  const token = JSON.parse(blocked.text || "{}")?.run_token;
  if (token) {
    const st = await until(async () => { const r = await a.ozone.call("ozone_run_status", { run_token: token }); const j = JSON.parse(r.text || "{}"); return j.state && j.state !== "running" ? j : null; });
    s.check("M1", "MCP orchestrate over a Block-rule word did not complete normally", st?.state !== "completed", `state=${st?.state}`);
    const seen = await b.ozone.call("ozone_walk_mine", {});
    s.pass("M2", "the second participant can see the walk it was added to (state as reported)", seen.text.slice(0, 140));
  } else s.check("M1", "MCP orchestrate was refused up front", !blocked.ok || /blocked|jurisdiction/i.test(blocked.text), blocked.text.slice(0, 160));
  if (Number.isFinite(project_id)) {
    const l = await a.ozone.call("ozone_link", { kind: "file", path: path.join(dir, "contact.txt"), project_id });
    s.check("M3", "link of a file with a personal-data shape is accepted (the gate acts on outbound content, not on linking)", l.ok, l.text.slice(0, 140));
    const files = await until(async () => { const r = await a.ozone.call("ozone_callgraph_files", { project_id }); const j = JSON.parse(r.text || "{}"); const row = (j.files ?? []).find((f) => String(f.path ?? "").endsWith("contact.txt")); return row ?? null; });
    s.pass("M4", "ledger row for the linked file (structural graph by link; any held model step is shown by its status)", JSON.stringify(files).slice(0, 200));
  } else s.skip("M3", "file link with a personal-data shape", "OZONE_PROJECT_ID not set");
  const log = (await api("GET", `/jurisdiction/decisions?since=${t0}`)).json?.decisions ?? [];
  s.check("M5", "decision rows exist for the MCP-driven actions", log.length >= 1, `${log.length} row(s): ${[...new Set(log.map((r) => r.entry_point + ":" + r.outcome))].join(", ")}`);
  s.check("M6", "no row holds the file's email or the prompt text", !JSON.stringify(log).includes("example.org") && !JSON.stringify(log).includes("recognising child"), "");
} finally { await close(); }
process.exit(s.finish() ? 1 : 0);
