// Ozone-Studio Terminal MCP — the 5th external capability (guide §4
// contract, operator directive 2026-09-28): a STRUCTURED, SECURED
// terminal exposed as an MCP tool, registered into the host's
// /mcp/tools and invoked through /mcp/call — so every command is
// metered (per-agent/day ledger), jurisdiction-gated, rippled into the
// graph, and wrapped in the insight envelope. Not a raw shell: a
// tool-call surface with an explicit allowlist.
//
// SECURITY MODEL (deliberate, not an afterthought):
// 1. ALLOWLIST REQUIRED — OZONE_TERMINAL_ALLOW (comma-separated command
//    prefixes, e.g. "git,cargo,ls,npm"). If unset/empty, terminal_exec
//    refuses EVERY command (honest unavailable) — the MCP never boots
//    wide-open.
// 2. First-token prefix match against the allowlist; shell operators
//    (; | && ` $()) are REJECTED — one command per call, no chaining
//    smuggled past the allowlist.
// 3. Per-command timeout (default 60s, hard cap 600s), output capped
//    at 100KB, cwd must exist.
// 4. Every call arrives through /mcp/call, so agent identity, metering,
//    gating, and the ripple are enforced HOST-side — this process never
//    trusts its caller.
//
// Endpoints: POST /call {tool, agent, input:{command, cwd?, timeout_secs?}}
//            GET  /status
// Env:       OZONE_TERMINAL_PORT (default 3215), OZONE_TERMINAL_ALLOW.

import { createServer } from "node:http";
import { spawn } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import fs from "node:fs";

const PORT = Number(process.env.OZONE_TERMINAL_PORT ?? 3215);
// PER-ROLE ALLOWLISTS (operator directive 2026-09-28: agents have roles —
// coordinators are a higher trust tier than everyone else, not identical):
// OZONE_TERMINAL_ROLES is JSON { "<agent>": ["cmd", ...], "*": [...] }.
// Resolution: exact agent match first, then "*" (the everyone-else tier,
// most restrictive), then legacy OZONE_TERMINAL_ALLOW (global), then LOCKED.
// The role comes from the call's own agent field — the same identity the
// host meters and gates on, so role trust and usage accountability share
// one identity.
const ROLES = (() => {
  const fromFile = process.env.OZONE_TERMINAL_ROLES_FILE;
  if (fromFile) {
    try { return JSON.parse(fs.readFileSync(fromFile, "utf8")); }
    catch (e) { console.error("[terminal-mcp] roles file unreadable:", e.message); }
  }
  try {
    const inline = JSON.parse(process.env.OZONE_TERMINAL_ROLES ?? "{}");
    if (Object.keys(inline).length > 0) return inline;
  } catch { /* fall through to the default file below */ }
  // Real launch-configuration trap, found live 2026-09-29: roles.json sits
  // right next to this file but was silently ignored (every agent LOCKED)
  // unless OZONE_TERMINAL_ROLES_FILE was explicitly set. Default to it when
  // present — still LOCKED if it's missing or invalid, per this MCP's own
  // "unconfigured = LOCKED" doctrine, just no longer locked when a real,
  // valid roles file is sitting right there unused.
  const defaultPath = path.join(path.dirname(fileURLToPath(import.meta.url)), "roles.json");
  try { return JSON.parse(fs.readFileSync(defaultPath, "utf8")); }
  catch { return {}; }
})();
const ALLOW = (process.env.OZONE_TERMINAL_ALLOW ?? "")
  .split(",")
  .map((s) => s.trim())
  .filter(Boolean);

function allowlistFor(agent) {
  if (agent && Array.isArray(ROLES[agent])) return ROLES[agent];
  if (Array.isArray(ROLES["*"])) return ROLES["*"];
  return ALLOW; // legacy global fallback; empty = locked
}
// Optional shared secret: when set, /call requires header
// `x-ozone-terminal-token: <OZONE_TERMINAL_TOKEN>` — defense against
// another LOCAL process calling :3215 directly to bypass Ozone's
// metering/review. Loopback binding + allowlist are the first layers;
// this is the third. Absent = token check off (loopback-only trust).
const TOKEN_SECRET = process.env.OZONE_TERMINAL_TOKEN ?? "";
const PLATFORM = `${process.platform}/${process.arch}`; // e.g. linux/x64
const FORBIDDEN = /[;|&`$><]/;
const startedAt = Date.now();

function exec_allowed(command, agent) {
  const allow = allowlistFor(agent);
  if (!allow.length) return { ok: false, reason: `no allowlist for agent '${agent ?? "?"}' — terminal_exec is locked for this role` };
  if (!command || typeof command !== "string") return { ok: false, reason: "missing command" };
  if (FORBIDDEN.test(command)) return { ok: false, reason: "shell operators (; | & ` $ < >) are rejected — one command per call" };
  const first = command.trim().split(/\s+/)[0];
  if (!allow.some((a) => first === a || first.startsWith(a))) {
    return { ok: false, reason: `command '${first}' is not in the allowlist for role of '${agent}' (${allow.join(", ")})` };
  }
  return { ok: true };
}

function run_command(command, cwd, timeoutSecs) {
  return new Promise((resolve) => {
    const timeoutMs = Math.min(Math.max(Number(timeoutSecs ?? 60), 1), 600) * 1000;
    const cwdAbs = cwd ? path.resolve(cwd) : process.cwd();
    if (!fs.existsSync(cwdAbs)) {
      resolve({ success: false, error: `cwd does not exist: ${cwdAbs}` });
      return;
    }
    const child = spawn(command, { shell: true, cwd: cwdAbs, timeout: timeoutMs });
    let stdout = "";
    let stderr = "";
    const cap = 100 * 1024;
    child.stdout.on("data", (d) => { if (stdout.length < cap) stdout += d; });
    child.stderr.on("data", (d) => { if (stderr.length < cap) stderr += d; });
    child.on("error", (err) => resolve({ success: false, error: err.message }));
    child.on("close", (code, signal) => resolve({
      success: code === 0,
      exit_code: code,
      killed_by_timeout: signal === "SIGTERM",
      stdout: stdout.slice(0, cap),
      stderr: stderr.slice(0, cap),
    }));
  });
}

function reply(res, code, body) {
  res.writeHead(code, { "Content-Type": "application/json" });
  res.end(JSON.stringify(body));
}

const server = createServer((req, res) => {
  if (req.method === "GET" && (req.url ?? "").startsWith("/status")) {
    reply(res, 200, {
      tool: "terminal-mcp",
      started_at: new Date(startedAt).toISOString(),
      allowlist: ALLOW,
      locked: ALLOW.length === 0,
    });
    return;
  }
  if (req.method !== "POST" || !(req.url ?? "").startsWith("/call")) {
    reply(res, 404, { error: "POST /call or GET /status only" });
    return;
  }
  if (TOKEN_SECRET) {
    const got = req.headers["x-ozone-terminal-token"] ?? "";
    if (got !== TOKEN_SECRET) {
      reply(res, 401, { success: false, error: "invalid or missing x-ozone-terminal-token" });
      return;
    }
  }
  const chunks = [];
  let size = 0;
  req.on("data", (c) => { size += c.length; if (size > 1024 * 1024) req.destroy(); else chunks.push(c); });
  req.on("end", async () => {
    let tool = "";
    let input = {};
    let body = {};
    try {
      body = JSON.parse(Buffer.concat(chunks).toString("utf8") || "{}");
      tool = String(body?.tool ?? "");
      input = body?.input ?? {};
    } catch {
      reply(res, 400, { success: false, error: "invalid JSON body" });
      return;
    }
    if (tool === "terminal_status") {
      reply(res, 200, {
        success: true,
        output: {
          tool: "terminal-mcp",
          platform: PLATFORM,
          allowlist_legacy_global: ALLOW,
          roles_configured: Object.keys(ROLES),
          locked: ALLOW.length === 0 && Object.keys(ROLES).length === 0,
          token_required: Boolean(TOKEN_SECRET),
          uptime_secs: Math.round((Date.now() - startedAt) / 1000),
        },
      });
      return;
    }
    if (tool !== "terminal_exec") {
      reply(res, 200, { success: false, error: `unknown terminal tool '${tool}' (terminal_exec | terminal_status)` });
      return;
    }
    const gate = exec_allowed(String(input.command ?? ""), String(body?.agent ?? ""));
    if (!gate.ok) {
      reply(res, 200, { success: false, error: gate.reason });
      return;
    }
    const result = await run_command(
      String(input.command),
      input.cwd ? String(input.cwd) : undefined,
      input.timeout_secs
    );
    reply(res, 200, { success: result.success, output: result });
  });
});

server.on("error", (err) => {
  if (err.code === "EADDRINUSE") {
    console.error(`[terminal-mcp] :${PORT} already in use — another instance serves /call; skipping`);
    return;
  }
  console.error("[terminal-mcp] server error:", err);
});

server.listen(PORT, "127.0.0.1", () => {
  console.error(
    `[terminal-mcp] /call on 127.0.0.1:${PORT} | allowlist: ${ALLOW.length ? ALLOW.join(", ") : "LOCKED (no OZONE_TERMINAL_ALLOW)"}`
  );
});
