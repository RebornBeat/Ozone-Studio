// Host loopback credential for the engine bridge — the TypeScript twin of tools/shared/guard.mjs (same token lookup, same refusal shapes).
//
// WHY (bridges incorporation, 2026-11-12): the bridge's /execute and /call run engine commands (roblox_run_code, unreal_run_python,
// blender_run_python execute arbitrary code in the engine). They were open to ANY local process. On a strict host (OZONE_REQUIRE_SESSION=1 or
// OZONE_MCP_REQUIRE_INTERNAL=1) they now accept only calls that carry the host's internal token (`x-oz-internal`), exactly like every
// other Ozone MCP server; and the bridge's own calls INTO the host (`/pipelines/register`, which is a protected route) carry the same token.
//
// Token lookup (first hit wins): OZONE_INTERNAL_TOKEN, OZONE_INTERNAL_TOKEN_FILE, $OZONE_DATA_DIR/.internal_token,
// <repo>/target/release/zsei_data/.internal_token, <repo>/zsei_data/.internal_token. The token is never logged or returned.

import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import type { IncomingMessage, ServerResponse } from "node:http";

const here = path.dirname(fileURLToPath(import.meta.url));
// src/ozone-auth.ts compiles to dist/ozone-auth.js: <repo>/tools/engine-bridge/dist -> repo is three levels up.
const repo = path.resolve(here, "..", "..", "..");
let cached: string | null = null;

export function internalToken(): string {
  if (cached) return cached;
  const env = process.env["OZONE_INTERNAL_TOKEN"];
  if (env) return (cached = env);
  const files = [
    process.env["OZONE_INTERNAL_TOKEN_FILE"],
    process.env["OZONE_DATA_DIR"] ? path.join(process.env["OZONE_DATA_DIR"] as string, ".internal_token") : undefined,
    path.join(repo, "target", "release", "zsei_data", ".internal_token"),
    path.join(repo, "zsei_data", ".internal_token")
  ].filter((f): f is string => Boolean(f));
  for (const f of files) {
    try {
      const v = fs.readFileSync(f, "utf8").trim();
      if (v) return (cached = v);
    } catch {
      /* next */
    }
  }
  return "";
}

export function guardRequired(): boolean {
  return process.env["OZONE_MCP_REQUIRE_INTERNAL"] === "1" || process.env["OZONE_REQUIRE_SESSION"] === "1";
}

/** Headers for a bridge -> host call: JSON plus `x-oz-internal` when a token exists. */
export function internalHeaders(extra: Record<string, string> = {}): Record<string, string> {
  const h: Record<string, string> = { "Content-Type": "application/json", ...extra };
  const t = internalToken();
  if (t) h["x-oz-internal"] = t;
  return h;
}

/** true = proceed. false = a refusal has already been written to `res` (fail closed: no token available = refuse). */
export function guardOk(req: IncomingMessage, res: ServerResponse): boolean {
  if (!guardRequired()) return true;
  const need = internalToken();
  if (!need) {
    res.writeHead(503, { "Content-Type": "application/json" });
    res.end(JSON.stringify({ success: false, error: "engine bridge loopback guard is on but no internal token is available (set OZONE_INTERNAL_TOKEN or OZONE_DATA_DIR)" }));
    return false;
  }
  const got = String(req.headers["x-oz-internal"] ?? "");
  const a = Buffer.from(got);
  const b = Buffer.from(need);
  if (a.length === b.length && crypto.timingSafeEqual(a, b)) return true;
  res.writeHead(401, { "Content-Type": "application/json" });
  res.end(JSON.stringify({ success: false, error: "this bridge only accepts calls routed through the Ozone host (/mcp/call or pipeline execute): missing or invalid x-oz-internal" }));
  return false;
}
