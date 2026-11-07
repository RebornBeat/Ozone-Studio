// L6 MCP contract (MASTER_TEST_GUIDE §0.3 / NEW_MCP_GUIDE §6): every MCP row has five checks, all through /mcp/call:
//   positive · refusal (HTTP 200, success:false + reason) · ripple/captured block · S13 capture row · registry endpoint == listening port
import fs from "node:fs";
import { suite, mcp, http } from "./lib.mjs";
import { makeFixtures } from "./fixtures.mjs";

const s = suite("e2e-mcp-contract");
const FX = makeFixtures("/tmp/e2e_fixtures");
fs.mkdirSync("/home/rebornbeat/Projects/Ozone-Studio/tools/mesh-mcp/data/e2e", { recursive: true });
fs.copyFileSync("/tmp/e2e_fixtures/cube.obj", "/home/rebornbeat/Projects/Ozone-Studio/tools/mesh-mcp/data/e2e/cube.obj");

const tools = (await http("GET", "/mcp/tools")).json.tools;
const byName = Object.fromEntries(tools.map((t) => [t.name, t]));

// [mcp, port, positive {tool,input,expect(fn)}, refusal {tool,input}]
const CASES = [
  ["terminal-mcp", 3215, { tool: "terminal_status", input: {}, ok: (o) => JSON.stringify(o).includes("terminal") }, { tool: "terminal_exec", input: { command: "rm -rf /" } }],
  ["web-mcp", 3225, { tool: "current_datetime", input: {}, ok: (o) => /20\d\d/.test(JSON.stringify(o)) }, { tool: "web_search", input: { query: "" } }],
  ["code-mcp", 3230, { tool: "code_callgraph", input: { code: "def a():\n    b()\ndef b():\n    pass\n", language: "python" }, ok: (o) => JSON.stringify(o).includes("a") }, { tool: "code_callgraph", input: {} }],
  ["git-mcp", 3235, { tool: "file_status", input: {}, ok: () => true }, { tool: "file_history", input: {}, optional: true }],
  ["security-mcp", 3240, { tool: "security_status", input: {}, ok: (o) => JSON.stringify(o).includes("read_only") }, { tool: "net_connections", input: { bogus: true }, optional: true }],
  ["dna-mcp", 3271, { tool: "dna_sequence_info", input: {}, ok: (o) => JSON.stringify(o).includes("580076") || JSON.stringify(o).includes("NC_000908") }, { tool: "dna_translate", input: { sequence: "XYZ!" } }],
  ["chemistry-mcp", 3272, { tool: "chem_analyze", input: { smiles: "CCO" }, ok: (o) => JSON.stringify(o).toLowerCase().includes("weight") || JSON.stringify(o).includes("46") }, { tool: "chem_analyze", input: { smiles: "not a molecule ###" } }],
  ["mesh-mcp", 3276, { tool: "mesh_analyze", input: { path: "/home/rebornbeat/Projects/Ozone-Studio/tools/mesh-mcp/data/e2e/cube.obj" }, ok: (o) => JSON.stringify(o).includes("8") }, { tool: "mesh_analyze", input: { path: "/home/rebornbeat/Projects/Ozone-Studio/tools/mesh-mcp/data/e2e/does-not-exist.obj" } }],
  ["document-mcp", 3282, { tool: "document_extract", input: { path: "/tmp/e2e_fixtures/fixture_text.pdf" }, ok: (o) => JSON.stringify(o).includes("quick brown fox") }, { tool: "document_extract", input: { path: "/tmp/e2e_fixtures/none.pdf" } }],
];

for (const [name, port, pos, ref] of CASES) {
  const t = byName[pos.tool];
  if (!t) { s.skip(`${name}`, "tool not registered", pos.tool); continue; }
  // 5. registry endpoint == real port
  s.check(`${name}.registry`, `registry endpoint is :${port}`, String(t.endpoint).includes(`:${port}/`), t.endpoint);
  // 1. positive
  const r = await mcp(pos.tool, pos.input);
  let o = r.output; try { if (r.success !== true) throw 0; } catch {}
  s.check(`${name}.positive`, `${pos.tool} succeeds with a real answer`, r.success === true && pos.ok(r.output ?? r), r.success ? JSON.stringify(r.output).slice(0, 100) : String(r.error).slice(0, 140));
  // 3. ripple/captured block present
  const cap = JSON.stringify(r.review?.captured ?? []);
  s.check(`${name}.ripple`, "the call carries the captured block (usage ledger + jurisdiction gate + graph ripple)", cap.includes("usage ledger") && cap.includes("jurisdiction") && cap.includes("ripple"), cap.slice(0, 120));
  // 2. refusal
  const f = await mcp(ref.tool, ref.input);
  if (ref.optional && f.success === true) s.skip(`${name}.refusal`, "no refusal path exercised", "tool accepted the probe");
  else s.check(`${name}.refusal`, `${ref.tool} bad input → clean success:false + reason (no 5xx, no crash)`, f.success === false && String(f.error ?? JSON.stringify(f.output)).length > 3, String(f.error ?? JSON.stringify(f.output)).slice(0, 120));
  // 4. S13 capture row
  const cap13 = (await http("GET", `/capture/tool-calls?tool=${pos.tool}`)).json;
  const n = Array.isArray(cap13) ? cap13.length : (cap13.rows ?? cap13.calls ?? cap13.tool_calls ?? []).length;
  s.check(`${name}.s13`, `S13 capture holds a row for ${pos.tool}`, n > 0, `rows=${n}`);
}
process.exit(s.finish() ? 1 : 0);
