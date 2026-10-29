// Register browser-mcp + browser-farm-mcp tools into the Ozone host registry
// (docs/NEW_MCP_GUIDE.md §1.4). Re-registration REPLACES and bumps server_version;
// the registry persists ({data_dir}/mcp_tool_registry.json), so re-run only when an endpoint changes.
//   node tools/register-browser-tools.mjs [browser|farm|all]
// Env: OZONE_HOST (http://127.0.0.1:50051), OZONE_BROWSER_PORT (3250), OZONE_FARM_PORT (3252),
//      OZONE_INTERNAL_TOKEN / OZONE_INTERNAL_TOKEN_FILE / OZONE_DATA_DIR (strict-session hosts)
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HOST = process.env.OZONE_HOST ?? "http://127.0.0.1:50051";
const PLATFORM = `platform:${process.platform}/${process.arch}`;
const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
function internalToken() {
  if (process.env.OZONE_INTERNAL_TOKEN) return process.env.OZONE_INTERNAL_TOKEN;
  for (const c of [process.env.OZONE_INTERNAL_TOKEN_FILE, process.env.OZONE_DATA_DIR && path.join(process.env.OZONE_DATA_DIR, ".internal_token"),
    path.join(repo, "target", "release", "zsei_data", ".internal_token"), path.join(repo, "zsei_data", ".internal_token")].filter(Boolean)) {
    try { const v = fs.readFileSync(c, "utf8").trim(); if (v) return v; } catch { /* next */ }
  }
  return "";
}
const FAMILIES = {
  browser: { port: Number(process.env.OZONE_BROWSER_PORT ?? 3250), version: "0.1.0", tools: {
    browser_status: "browser MCP posture: locked?, allowlist, sessions",
    browser_open: "open an isolated headless browser session on an allowlisted URL",
    browser_navigate: "navigate a session to an allowlisted URL",
    browser_snapshot: "perceive the page: semantic outline + numbered interactive elements (untrusted data)",
    browser_click: "click an element by snapshot index / selector / x,y",
    browser_type: "type into an element (optional clear/submit)",
    browser_scroll: "scroll the page",
    browser_screenshot: "PNG screenshot -> file path for image-modality ingest",
    browser_eval: "evaluate JS in the page (disabled unless OZONE_BROWSER_ALLOW_EVAL=1)",
    browser_console: "recent console/exception lines (untrusted)",
    browser_close: "close a session",
  } },
  farm: { port: Number(process.env.OZONE_FARM_PORT ?? 3252), version: "0.1.0", tools: {
    farm_status: "Browser-Farm MCP posture + per-farm health",
    farm_profiles: "list farm profiles (secrets stripped)", farm_profile: "one farm profile",
    farm_profile_start: "start a farm profile", farm_profile_pause: "pause a farm profile", farm_profile_stop: "stop a farm profile",
    farm_navigate: "steer a farm profile to an allowlisted URL", farm_screenshot: "farm profile screenshot -> local file",
    farm_metrics: "profile memory/cpu/uptime", farm_state_get: "read farm shared state", farm_state_set: "write farm shared state (object)",
    farm_command: "issue a COMMAND to the Command Center profile via shared state", farm_result: "read the Command Center's RESULT record",
  } },
};
const which = process.argv[2] ?? "all";
const tok = internalToken();
const headers = { "Content-Type": "application/json", ...(tok ? { "x-oz-internal": tok } : {}) };
let ok = 0, bad = 0;
for (const [fam, f] of Object.entries(FAMILIES)) {
  if (which !== "all" && which !== fam) continue;
  for (const [name, desc] of Object.entries(f.tools)) {
    try {
      const r = await fetch(`${HOST}/mcp/tools/register`, { method: "POST", headers, body: JSON.stringify({ name, transport: "http", endpoint: `http://127.0.0.1:${f.port}/call`, capabilities: [PLATFORM, desc], server_version: f.version }) });
      const j = await r.json().catch(() => ({}));
      if (r.ok && j.success !== false) { ok++; console.error(`  ✓ ${name}${j.replaced ? " (replaced)" : ""}`); } else { bad++; console.error(`  ✗ ${name}: HTTP ${r.status} ${j.error ?? ""}${r.status === 401 ? " — strict session host: needs x-oz-internal (build with the internal-token code + OZONE_DATA_DIR) or a session" : ""}`); }
    } catch (e) { bad++; console.error(`  ✗ ${name}: ${e.message}`); }
  }
}
console.error(`registered ${ok}, failed ${bad} @ ${HOST}`);
process.exit(bad ? 1 : 0);
