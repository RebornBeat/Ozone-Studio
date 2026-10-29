// browser-farm-mcp CONTRACT test against a MOCK farm.
// The mock mimics the REAL Browser-Farm API shapes read from server/src/server.py
// (X-API-Key auth, /health, /profiles, /profiles/{id}/{start|pause|stop|navigate?url=|screenshot|metrics},
// /state/{key}, /screenshots/{file}). It is NOT a live Browser-Farm: the farm needs PostgreSQL +
// Playwright + Xvfb (see ../../../Browser-Farm/README.md). A live acceptance run is a separate step.
import http from "node:http";
import { spawn } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const KEY = "bf_testkey_" + Math.random().toString(36).slice(2);
const state = {}; const log = [];
const PNG = Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==", "base64");
const profiles = [{ id: "p1", name: "worker-1", mode: "automated", status: "running", proxy_id: "px1", scripts: [], browser_engine: "chromium", os_fingerprint: "linux", proxy_password: "SECRET-SHOULD-NOT-LEAK", account_password: "NOPE" },
                  { id: "cc", name: "cc", mode: "command_center", status: "running" }];
const farm = http.createServer((req, res) => {
  const u = new URL(req.url, "http://x"); log.push(`${req.method} ${u.pathname}${u.search}`);
  const j = (c, b) => { res.writeHead(c, { "Content-Type": "application/json" }); res.end(JSON.stringify(b)); };
  if (req.headers["x-api-key"] !== KEY) return j(401, { detail: "Invalid API key" });
  const body = []; req.on("data", (c) => body.push(c)); req.on("end", () => {
    const b = body.length ? JSON.parse(Buffer.concat(body)) : null; let m;
    if (u.pathname === "/health") return j(200, { status: "online", version: "1.0.0", max_contexts: 20, current_contexts: 1 });
    if (u.pathname === "/profiles") return j(200, profiles);
    if ((m = u.pathname.match(/^\/profiles\/([^/]+)$/))) { const p = profiles.find((x) => x.id === m[1]); return p ? j(200, p) : j(404, { detail: "nope" }); }
    if ((m = u.pathname.match(/^\/profiles\/([^/]+)\/(start|pause|stop|navigate|screenshot|metrics)$/))) {
      if (m[2] === "screenshot") return j(200, { id: "p1_1", url: "/screenshots/p1_1.png" });
      if (m[2] === "metrics") return j(200, { memory_mb: 512, cpu_percent: 3.2 });
      if (m[2] === "navigate") return j(200, { status: "ok", url: u.searchParams.get("url") });
      return j(200, { status: m[2] });
    }
    if (u.pathname === "/screenshots/p1_1.png") { res.writeHead(200, { "Content-Type": "image/png" }); return res.end(PNG); }
    if ((m = u.pathname.match(/^\/state\/(.+)$/))) { const k = decodeURIComponent(m[1]); if (req.method === "POST") { state[k] = b; return j(200, { status: "ok" }); } return j(200, state[k] ?? {}); }
    j(404, { detail: "not found" });
  });
});
await new Promise((r) => farm.listen(8799, "127.0.0.1", r));
const keyFile = path.join(os.tmpdir(), "bf-test-key-" + process.pid); fs.writeFileSync(keyFile, KEY + "\n", { mode: 0o600 });
const start = (port, env) => spawn("node", ["server.mjs"], { env: { ...process.env, OZONE_FARM_PORT: String(port), ...env }, stdio: "ignore" });
const mcp = start(3253, { OZONE_FARM_URL: "http://127.0.0.1:8799", OZONE_FARM_API_KEY_FILE: keyFile, OZONE_FARM_ALLOW_HOSTS: "example.test", OZONE_FARM_SHOT_DIR: path.join(os.tmpdir(), "bf-shots") });
const ro = start(3254, { OZONE_FARM_URL: "http://127.0.0.1:8799", OZONE_FARM_API_KEY_FILE: keyFile, OZONE_FARM_READONLY: "1", OZONE_FARM_ALLOW_HOSTS: "example.test" });
const locked = start(3255, {});
const nokey = start(3256, { OZONE_FARM_URL: "http://127.0.0.1:8799" });
await new Promise((r) => setTimeout(r, 1200));
const call = async (port, tool, input = {}) => (await fetch(`http://127.0.0.1:${port}/call`, { method: "POST", body: JSON.stringify({ tool, agent: "selftest", input }) })).json();
let pass = 0, fail = 0; const ok = (n, c, x = "") => { c ? pass++ : fail++; console.log(`${c ? "PASS" : "FAIL"}  ${n}${x ? "  — " + x : ""}`); };
try {
  let r = await call(3255, "farm_profiles"); ok("locked-by-default (no farm configured)", !r.success && /LOCKED/.test(r.error), r.error);
  r = await call(3256, "farm_profiles"); ok("refuses farm with no API key", !r.success && /no API key/.test(r.error), r.error);
  r = await call(3253, "farm_status"); ok("status reaches farm, never reveals the key", r.success && r.output.farms.default.reachable && !JSON.stringify(r).includes(KEY), JSON.stringify(r.output.farms.default.health));
  r = await call(3253, "farm_profiles"); const ps = r.output?.untrusted_farm_data?.profiles;
  ok("lists profiles, wrapped as untrusted", r.success && ps?.length === 2);
  ok("profile output strips proxy/account secrets", !JSON.stringify(r).includes("SECRET-SHOULD-NOT-LEAK") && !JSON.stringify(r).includes("NOPE"));
  r = await call(3253, "farm_profile_start", { profile_id: "p1" }); ok("start profile", r.success && r.output.result.status === "start");
  r = await call(3253, "farm_profile_start", { profile_id: "../etc" }); ok("malformed profile_id refused", !r.success && /malformed/.test(r.error), r.error);
  r = await call(3253, "farm_navigate", { profile_id: "p1", url: "https://example.test/a" }); ok("navigate allowlisted host", r.success && r.output.result.url === "https://example.test/a");
  r = await call(3253, "farm_navigate", { profile_id: "p1", url: "https://evil.test/" }); ok("navigate off-allowlist refused", !r.success && /allowlist/.test(r.error), r.error);
  r = await call(3253, "farm_navigate", { profile_id: "p1", url: "javascript:alert(1)" }); ok("navigate javascript: refused", !r.success && /scheme/.test(r.error), r.error);
  r = await call(3253, "farm_screenshot", { profile_id: "p1" }); ok("screenshot downloaded locally", r.success && fs.existsSync(r.output.path) && r.output.bytes === PNG.length, r.output?.path);
  r = await call(3253, "farm_command", { target: "command-center", type: "ping" });
  ok("farm_command writes the CC command key", r.success && state["ozone~commands~command-center"]?.type === "ping", JSON.stringify(r.output?.issued));
  state["ozone~results~command-center"] = { id: r.output.issued.id, ok: true, output: { pong: true } };
  r = await call(3253, "farm_result", {}); ok("farm_result reads the CC result key", r.success && r.output.untrusted_farm_data.result.output.pong === true);
  r = await call(3253, "farm_command", { type: "rm -rf" }); ok("command type must be snake_case", !r.success && /snake_case/.test(r.error), r.error);
  r = await call(3253, "farm_command", { type: "go", url: "https://evil.test/" }); ok("command cannot smuggle a URL past the allowlist", !r.success && /allowlist/.test(r.error), r.error);
  r = await call(3253, "farm_state_set", { key: "a/b", value: [1] }); ok("state value must be an object", !r.success, r.error);
  r = await call(3253, "farm_script_run", {}); ok("script execution is NOT a tool", !r.success && /unknown farm tool/.test(r.error), r.error);
  r = await call(3254, "farm_profiles"); ok("read-only mode still reads", r.success);
  r = await call(3254, "farm_profile_stop", { profile_id: "p1" }); ok("read-only mode refuses mutation", !r.success && /READONLY/.test(r.error), r.error);
  r = await call(3254, "farm_command", { type: "ping" }); ok("read-only mode refuses commands", !r.success && /READONLY/.test(r.error));
} finally { for (const p of [mcp, ro, locked, nokey]) p.kill("SIGKILL"); farm.close(); fs.rmSync(keyFile, { force: true }); }
console.log(`\n${pass} passed, ${fail} failed (contract test vs MOCK farm — not a live Browser-Farm)`);
process.exit(fail ? 1 : 0);
