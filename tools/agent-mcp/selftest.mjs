#!/usr/bin/env node
// agent-mcp selftest — refusal paths FIRST (CREATING_NEW_MCPS_GUIDE §6:
// "Refusal path TESTED, not just the happy path"). This skeleton's
// happy-path capability work is pending by design, so the selftest
// asserts the HONEST refusals + the status contract. Run:
//   node tools/agent-mcp/selftest.mjs

import { spawn } from "node:child_process";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

const PORT = 3483;
const BASE = `http://127.0.0.1:${PORT}`;
let passed = 0, failed = 0;
const check = (name, ok, detail = "") => {
  if (ok) { passed++; console.log(`  ✓ ${name}`); }
  else { failed++; console.log(`  ✗ ${name} ${detail}`); }
};
const call = async (tool, input = {}) => {
  const r = await fetch(`${BASE}/call`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ tool, agent: "selftest", input, context: null }),
  });
  return r.json();
};

const server = spawn(process.execPath, [
  path.join(path.dirname(new URL(import.meta.url).pathname), "server.mjs"),
], {
  env: {
    ...process.env,
    OZONE_AGENT_PORT: String(PORT),
    OZONE_AGENT_ALLOW_HOSTS: "example.com",
    OZONE_AGENT_TOKEN: "selftest-token",
  },
  stdio: ["ignore", "ignore", "pipe"],
});
await new Promise((r) => setTimeout(r, 1200));

try {
  // 1. Shared-secret gate: bad token refused at the door.
  const bad = await (await fetch(`${BASE}/call`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: "{}",
  })).json();
  check("token gate: bad/missing token refused", bad.success === false && /token/i.test(bad.error));

  // Authenticated calls below.
  const authed = { "content-type": "application/json", "x-ozone-agent-token": "selftest-token" };
  const post = async (body) => (await fetch(`${BASE}/call`, { method: "POST", headers: authed, body: JSON.stringify(body) })).json();

  // 2. Unknown tool refused.
  const unk = await post({ tool: "agent_remote_control", input: {} });
  check("unknown tool refused", unk.success === false && /unknown tool/.test(unk.error));

  // 3. Nav allowlist: non-allowlisted host refused (AG2 shape).
  const nav = await post({ tool: "agent_navigate", input: { url: "https://evil.example.net/" } });
  check("navigate to non-allowlisted host refused (AG2)", nav.success === false && /LOCKED/.test(nav.error));

  // 4. Eval gate: exec script refused without OZONE_AGENT_ALLOW_EVAL (AG3).
  const ev = await post({ tool: "agent_exec_script", input: { expression: "1+1" } });
  check("exec_script LOCKED without eval opt-in (AG3)", ev.success === false && /ALLOW_EVAL/.test(ev.error));

  // 5. Harvest-pending honesty: an allowed tool answers honestly, not mock.
  const click = await post({ tool: "agent_click", input: { selector: "#q" } });
  check("click answers harvest-pending honestly (no mock success)", click.success === false && /harvest pending/.test(click.error));

  // 6. Status contract: backend, locked, tool list.
  const st = await (await fetch(`${BASE}/status`)).json();
  check("status: backend=farm default", st.backend === "farm");
  check("status: 16 tools declared", Array.isArray(st.tools) && st.tools.length === 16);
  check("status: core not wired", st.core_wired === false);
  check("status: nav allowlist active", st.locked === false && st.allow_hosts.includes("example.com"));
} finally {
  server.kill();
}
console.log(`\n${passed} passed, ${failed} failed`);
process.exit(failed ? 1 : 0);
