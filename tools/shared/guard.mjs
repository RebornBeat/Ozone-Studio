// Shared loopback guard for every Ozone MCP server (2026-11-02 audit).
//
// WHY: each MCP binds 127.0.0.1 and trusted the host's /mcp/call for the session
// gate, jurisdiction gate and usage ledger. Anything that could reach the port
// directly (another local process, a browser-origin request to localhost) skipped
// all of that. The host now sends its internal token (`x-oz-internal`, the same
// file-permission-boundary token the session gate accepts) on every tool call;
// this guard rejects calls without it.
//
// WHEN ENFORCED: OZONE_MCP_REQUIRE_INTERNAL=1, or automatically on strict-session
// hosts (OZONE_REQUIRE_SESSION=1). Otherwise a no-op, so a dev box with an older
// host binary keeps working. Enforcement is "fail closed": if enforcement is on and
// no token can be found, calls are refused with 503 (never silently allowed).
//
// Token lookup (first hit wins): OZONE_INTERNAL_TOKEN, OZONE_INTERNAL_TOKEN_FILE,
// $OZONE_DATA_DIR/.internal_token, <repo>/target/release/zsei_data/.internal_token,
// <repo>/zsei_data/.internal_token.

import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { fileURLToPath } from "node:url";

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
let cachedToken = null;

function internalToken() {
  if (cachedToken) return cachedToken;
  if (process.env.OZONE_INTERNAL_TOKEN) return (cachedToken = process.env.OZONE_INTERNAL_TOKEN);
  const files = [
    process.env.OZONE_INTERNAL_TOKEN_FILE,
    process.env.OZONE_DATA_DIR && path.join(process.env.OZONE_DATA_DIR, ".internal_token"),
    path.join(repo, "target", "release", "zsei_data", ".internal_token"),
    path.join(repo, "zsei_data", ".internal_token"),
  ].filter(Boolean);
  for (const f of files) {
    try {
      const v = fs.readFileSync(f, "utf8").trim();
      if (v) return (cachedToken = v);
    } catch { /* next */ }
  }
  return "";
}

export function guardRequired() {
  return process.env.OZONE_MCP_REQUIRE_INTERNAL === "1" || process.env.OZONE_REQUIRE_SESSION === "1";
}

/** true = proceed. false = a refusal has already been written to `res`. */
export function guardOk(req, res) {
  if (!guardRequired()) return true;
  const need = internalToken();
  if (!need) {
    res.writeHead(503, { "Content-Type": "application/json" });
    res.end(JSON.stringify({ success: false, error: "MCP loopback guard is on but no internal token is available (set OZONE_INTERNAL_TOKEN or OZONE_DATA_DIR)" }));
    return false;
  }
  const got = String(req.headers["x-oz-internal"] ?? "");
  const a = Buffer.from(got);
  const b = Buffer.from(need);
  if (a.length === b.length && crypto.timingSafeEqual(a, b)) return true;
  res.writeHead(401, { "Content-Type": "application/json" });
  res.end(JSON.stringify({ success: false, error: "this MCP only accepts calls routed through the Ozone host (/mcp/call): missing or invalid x-oz-internal" }));
  return false;
}
