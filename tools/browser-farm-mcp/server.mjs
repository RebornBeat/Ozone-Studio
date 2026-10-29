// Ozone-Studio Browser-Farm MCP — the Ozone side of Browser-Farm's COMMAND
// CENTER (Pattern B bridge: docs/NEW_MCP_GUIDE.md §2, project
// ~/Projects/Browser-Farm). Browser-Farm is a separate project: a FastAPI +
// Playwright server that runs many isolated browser profiles (own Xvfb display,
// proxy, fingerprint, persistent profile) with a singleton COMMAND_CENTER
// profile that orchestrates the others through a shared-state API. This MCP is
// the hook that lets Ozone-Studio be the commander above it:
//
//   Ozone orchestrator ──/mcp/call──▶ this MCP ──X-API-Key REST──▶ Browser-Farm server(s)
//                                                       └─ shared state ◀──▶ CC profile script
//
// Ozone never executes farm scripts and never touches the account vault or
// proxy credentials. It (a) observes the farm, (b) starts/pauses/stops
// profiles, (c) steers a profile's page (navigate), (d) pulls screenshots into
// the image modality, and (e) talks to the Command Center profile by writing
// COMMAND records into the farm's shared state and reading RESULT records back
// (see cc_script_ozone_bridge.py for the CC-side consumer).
//
// SECURITY MODEL (before capability):
// 1. LOCKED unless a farm is configured: OZONE_FARM_URL + an API key from
//    OZONE_FARM_API_KEY_FILE (preferred) or OZONE_FARM_API_KEY. The key is never
//    returned, logged or echoed. Several farms: OZONE_FARM_SERVERS_FILE =
//    {"name": {"url": "...", "api_key_file": "..."}} and `server` in each call.
// 2. Remote farms must be https:// OR explicitly opted in (OZONE_FARM_ALLOW_INSECURE=1);
//    loopback/private-LAN http is allowed because the farm README's own
//    deployment is http on a LAN.
// 3. OZONE_FARM_READONLY=1 disables every mutating tool.
// 4. farm_navigate is bounded by OZONE_FARM_ALLOW_HOSTS (same semantics as
//    browser-mcp); unset => navigation refused. Farm profiles carry proxies and
//    anti-detection fingerprints, so WHERE an agent may point them is the
//    operator's call, not the model's.
// 5. NOT EXPOSED, deliberately: script create/run, account vault, proxy
//    registration, profile delete. Script execution on the farm is arbitrary
//    code execution; credentials never cross this boundary. Do those in the
//    Browser-Farm client.
// 6. Everything the farm returns is untrusted data (wrapped as such).
// Every call arrives through /mcp/call: identity, metering, jurisdiction gate,
// ripple are host-side.
//
// Tools: farm_status farm_profiles farm_profile farm_profile_start farm_profile_pause
//        farm_profile_stop farm_navigate farm_screenshot farm_metrics
//        farm_state_get farm_state_set farm_command farm_result
// Env:   OZONE_FARM_PORT (3281) + the OZONE_FARM_* vars above.

import { createServer } from "node:http";
import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";

const PORT = Number(process.env.OZONE_FARM_PORT ?? 3281);
const READONLY = process.env.OZONE_FARM_READONLY === "1";
const ALLOW_INSECURE = process.env.OZONE_FARM_ALLOW_INSECURE === "1";
const ALLOW_HOSTS = (process.env.OZONE_FARM_ALLOW_HOSTS ?? "").split(",").map((s) => s.trim().toLowerCase()).filter(Boolean);
const TOKEN_SECRET = process.env.OZONE_FARM_TOKEN ?? "";
const SHOT_DIR = process.env.OZONE_FARM_SHOT_DIR ?? path.join(process.cwd(), ".ozone-context", "farm-shots");
const startedAt = Date.now();
const OUT_CAP = 60 * 1024;

// ── farm registry (keys stay in this process) ──────────────────────────────
function readKey(fileVar, envVar, obj) {
  const f = obj?.api_key_file ?? process.env[fileVar];
  if (f) { try { return fs.readFileSync(f, "utf8").trim(); } catch { return ""; } }
  return (obj?.api_key ?? process.env[envVar] ?? "").trim();
}
function loadFarms() {
  const farms = {};
  const file = process.env.OZONE_FARM_SERVERS_FILE;
  if (file) {
    try { for (const [name, v] of Object.entries(JSON.parse(fs.readFileSync(file, "utf8")))) farms[name] = { url: String(v.url ?? "").replace(/\/+$/, ""), key: readKey("", "", v) }; }
    catch (e) { console.error("[browser-farm-mcp] servers file unreadable:", e.message); }
  }
  if (process.env.OZONE_FARM_URL) farms.default = { url: process.env.OZONE_FARM_URL.replace(/\/+$/, ""), key: readKey("OZONE_FARM_API_KEY_FILE", "OZONE_FARM_API_KEY") };
  return farms;
}
const FARMS = loadFarms();

function isPrivateHost(h) {
  return /^(localhost|127\.|10\.|192\.168\.|172\.(1[6-9]|2\d|3[01])\.|\[?::1\]?$)/.test(h);
}
function farmFor(input) {
  const names = Object.keys(FARMS);
  if (!names.length) throw new Error("browser-farm-mcp is LOCKED: no farm configured (OZONE_FARM_URL + OZONE_FARM_API_KEY_FILE, or OZONE_FARM_SERVERS_FILE)");
  const name = String(input.server ?? (FARMS.default ? "default" : names[0]));
  const f = FARMS[name];
  if (!f) throw new Error(`unknown farm '${name}' (configured: ${names.join(", ")})`);
  if (!f.key) throw new Error(`farm '${name}' has no API key configured (refusing unauthenticated calls)`);
  let u; try { u = new URL(f.url); } catch { throw new Error(`farm '${name}' has an invalid url`); }
  if (u.protocol !== "https:" && !isPrivateHost(u.hostname) && !ALLOW_INSECURE) throw new Error(`farm '${name}' is plain http on a non-private host — use https or set OZONE_FARM_ALLOW_INSECURE=1`);
  return { name, ...f };
}

async function farmFetch(f, method, pathname, { query, body, binary } = {}) {
  const u = new URL(f.url + pathname);
  for (const [k, v] of Object.entries(query ?? {})) u.searchParams.set(k, String(v));
  const res = await fetch(u, {
    method, headers: { "X-API-Key": f.key, ...(body ? { "Content-Type": "application/json" } : {}) },
    body: body ? JSON.stringify(body) : undefined, signal: AbortSignal.timeout(70000),
  });
  if (!res.ok) {
    let detail = ""; try { detail = (await res.json()).detail ?? ""; } catch { /* non-json */ }
    throw new Error(`farm '${f.name}' ${method} ${pathname} → HTTP ${res.status}${detail ? ": " + String(detail).slice(0, 200) : ""}`);
  }
  return binary ? Buffer.from(await res.arrayBuffer()) : await res.json();
}

// ── policy helpers ─────────────────────────────────────────────────────────
function mutating() { if (READONLY) throw new Error("OZONE_FARM_READONLY=1: mutating farm tools are disabled"); }
function navPolicy(raw) {
  let u; try { u = new URL(raw); } catch { throw new Error("not a valid absolute URL"); }
  if (u.protocol !== "http:" && u.protocol !== "https:") throw new Error(`scheme ${u.protocol} is refused (http/https only)`);
  if (!ALLOW_HOSTS.length) throw new Error("farm_navigate is LOCKED: OZONE_FARM_ALLOW_HOSTS is not configured");
  const h = u.hostname.toLowerCase();
  if (!(ALLOW_HOSTS.includes("*") || ALLOW_HOSTS.some((a) => h === a || h.endsWith("." + a)))) throw new Error(`host '${u.hostname}' is not in the farm navigation allowlist (${ALLOW_HOSTS.join(", ")})`);
}
const ID_RE = /^[A-Za-z0-9._:-]{1,80}$/;
function pid(input) { const id = String(input.profile_id ?? ""); if (!ID_RE.test(id)) throw new Error("profile_id missing or malformed"); return id; }
function stateKey(k) { const s = String(k ?? ""); if (!/^[A-Za-z0-9._:/-]{1,120}$/.test(s)) throw new Error("state key must match [A-Za-z0-9._:/-]{1,120}"); return s.replace(/\//g, "~"); }
const capText = (v) => { const j = JSON.stringify(v); return j.length > OUT_CAP ? { truncated: true, preview: j.slice(0, OUT_CAP) } : v; };
const untrusted = (v) => capText({ untrusted_farm_data: v });

// Profile records can hold proxy/account links — pass through only operational fields.
function safeProfile(p) {
  if (!p || typeof p !== "object") return p;
  const keep = ["id", "name", "mode", "status", "browser_engine", "os_fingerprint", "proxy_id", "scripts", "timezone", "locale", "memory_threshold_mb", "created_at", "last_warmed"];
  return Object.fromEntries(keep.filter((k) => k in p).map((k) => [k, p[k]]));
}

// ── tools ──────────────────────────────────────────────────────────────────
async function handle(tool, input) {
  if (tool === "farm_status") {
    const names = Object.keys(FARMS);
    const out = { tool: "browser-farm-mcp", started_at: new Date(startedAt).toISOString(), locked: names.length === 0, readonly: READONLY, navigation_hosts: ALLOW_HOSTS, token_required: Boolean(TOKEN_SECRET), farms: {} };
    for (const n of names) {
      const f = FARMS[n]; const e = { url: f.url, key_configured: Boolean(f.key) };
      if (f.key) { try { e.health = await farmFetch({ name: n, ...f }, "GET", "/health"); e.reachable = true; } catch (err) { e.reachable = false; e.error = String(err.message).slice(0, 160); } }
      out.farms[n] = e;
    }
    return out;
  }
  const f = farmFor(input);
  switch (tool) {
    case "farm_profiles": { const l = await farmFetch(f, "GET", "/profiles"); return untrusted({ farm: f.name, profiles: (Array.isArray(l) ? l : l.profiles ?? []).map(safeProfile) }); }
    case "farm_profile": return untrusted(safeProfile(await farmFetch(f, "GET", `/profiles/${pid(input)}`)));
    // The farm's start endpoint requires an `accounts` JSON body (a map) —
    // a bodyless POST fails FastAPI validation with HTTP 422 (found live on
    // the real EU farm, 2026-10-29; the mock never validated this).
    case "farm_profile_start": mutating(); return { farm: f.name, result: await farmFetch(f, "POST", `/profiles/${pid(input)}/start`, { body: {} }) };
    case "farm_profile_pause": mutating(); return { farm: f.name, result: await farmFetch(f, "POST", `/profiles/${pid(input)}/pause`) };
    case "farm_profile_stop": mutating(); return { farm: f.name, result: await farmFetch(f, "POST", `/profiles/${pid(input)}/stop`) };
    case "farm_navigate": mutating(); navPolicy(String(input.url ?? "")); return { farm: f.name, result: await farmFetch(f, "POST", `/profiles/${pid(input)}/navigate`, { query: { url: String(input.url) } }) };
    case "farm_metrics": return untrusted(await farmFetch(f, "GET", `/profiles/${pid(input)}/metrics`));
    case "farm_screenshot": {
      mutating(); // writes a file on the farm
      const id = pid(input);
      const shot = await farmFetch(f, "POST", `/profiles/${id}/screenshot`);
      const rel = String(shot.url ?? "");
      if (!/^\/screenshots\/[A-Za-z0-9._-]+\.png$/.test(rel)) throw new Error("farm returned an unexpected screenshot url");
      const bytes = await farmFetch(f, "GET", rel, { binary: true });
      fs.mkdirSync(SHOT_DIR, { recursive: true });
      const file = path.join(SHOT_DIR, `${f.name}-${id}-${Date.now()}.png`);
      fs.writeFileSync(file, bytes);
      return { farm: f.name, path: file, bytes: bytes.length, farm_id: shot.id, note: "feed to visual_describe / visual_ingest (image modality 102) for graph capture" };
    }
    // ── Command Center channel: shared state is the wire ───────────────────
    case "farm_state_get": return untrusted({ key: String(input.key), value: await farmFetch(f, "GET", `/state/${encodeURIComponent(stateKey(input.key))}`) });
    case "farm_state_set": mutating(); {
      const k = stateKey(input.key);
      if (typeof input.value !== "object" || input.value === null || Array.isArray(input.value)) throw new Error("value must be a JSON object");
      if (JSON.stringify(input.value).length > 32 * 1024) throw new Error("value too large (32 KB cap)");
      return { farm: f.name, result: await farmFetch(f, "POST", `/state/${encodeURIComponent(k)}`, { body: input.value }) };
    }
    case "farm_command": {
      // Writes ozone~commands~<target> = {id,type,args,issued_by,issued_at}; the CC script polls it.
      mutating();
      const target = String(input.target ?? "command-center");
      if (!ID_RE.test(target)) throw new Error("target malformed");
      const type = String(input.type ?? "");
      if (!/^[a-z][a-z0-9_]{1,40}$/.test(type)) throw new Error("command type must be snake_case (e.g. 'run_scenario', 'report')");
      if (input.url) navPolicy(String(input.url)); // a command may not smuggle a URL past the allowlist
      const rec = { id: crypto.randomBytes(6).toString("hex"), type, args: input.args ?? {}, issued_by: String(input.agent ?? "ozone"), issued_at: new Date().toISOString() };
      if (JSON.stringify(rec).length > 32 * 1024) throw new Error("command too large (32 KB cap)");
      await farmFetch(f, "POST", `/state/${encodeURIComponent(stateKey(`ozone/commands/${target}`))}`, { body: rec });
      return { farm: f.name, issued: rec, result_key: `ozone/results/${target}`, note: "CC script (cc_script_ozone_bridge.py) must be running on the Command Center profile to consume this" };
    }
    case "farm_result": {
      const target = String(input.target ?? "command-center");
      if (!ID_RE.test(target)) throw new Error("target malformed");
      return untrusted({ target, result: await farmFetch(f, "GET", `/state/${encodeURIComponent(stateKey(`ozone/results/${target}`))}`) });
    }
    default:
      throw new Error(`unknown farm tool '${tool}'`);
  }
}

function reply(res, code, body) { res.writeHead(code, { "Content-Type": "application/json" }); res.end(JSON.stringify(body)); }

const server = createServer((req, res) => {
  const url = req.url ?? "";
  if (req.method === "GET" && url.startsWith("/status")) { handle("farm_status", {}).then((o) => reply(res, 200, o)); return; }
  if (req.method !== "POST" || !url.startsWith("/call")) { reply(res, 404, { error: "POST /call or GET /status" }); return; }
  if (TOKEN_SECRET && req.headers["x-ozone-farm-token"] !== TOKEN_SECRET) { reply(res, 401, { success: false, error: "missing/invalid x-ozone-farm-token" }); return; }
  const chunks = []; let size = 0;
  req.on("data", (c) => { size += c.length; if (size > 1024 * 1024) req.destroy(); else chunks.push(c); });
  req.on("end", async () => {
    let body;
    try { body = JSON.parse(Buffer.concat(chunks).toString("utf8") || "{}"); } catch { reply(res, 400, { success: false, error: "invalid JSON body" }); return; }
    try { reply(res, 200, { success: true, output: await handle(String(body?.tool ?? ""), { ...(body?.input ?? {}), agent: body?.agent }) }); }
    catch (e) { reply(res, 200, { success: false, error: String(e?.message ?? e) }); }
  });
});
server.on("error", (err) => {
  if (err.code === "EADDRINUSE") { console.error(`[browser-farm-mcp] :${PORT} already in use — another instance serves /call; skipping`); return; }
  console.error("[browser-farm-mcp] server error:", err);
});
server.listen(PORT, "127.0.0.1", () => {
  const names = Object.keys(FARMS);
  console.error(`[browser-farm-mcp] /call on 127.0.0.1:${PORT} | farms: ${names.length ? names.join(", ") : "LOCKED (none configured)"} | readonly: ${READONLY} | nav hosts: ${ALLOW_HOSTS.length ? ALLOW_HOSTS.join(", ") : "LOCKED"}`);
});
