// Ozone-Studio Zoom MCP — Universal Order Stage 3 connector
// (docs/UNIVERSAL_ORDER_GUIDE.md §3: "Zoom MCP (tools/zoom-mcp/): same
// shape [as gcal-mcp] — zoom_list_meetings, zoom_create_meeting,
// zoom_join_url; meetings sync as meeting tasks with meeting_url set.").
// Mirrors tools/gcal-mcp/server.mjs's exact real structure (scoped and
// designed in docs/MCP_TESTING_AND_COORDINATION_GUIDE.md before being
// built here) — external events land as TASKS with external_ref
// provenance, the order never stores a foreign schema.
//
// SECURITY / LOCKED-BY-DEFAULT (same doctrine as gcal-mcp,
// NEW_MCP_GUIDE.md §6): requires OZONE_ZOOM_CLIENT_ID +
// OZONE_ZOOM_CLIENT_SECRET (a Zoom OAuth app's credentials). Without
// them: locked — every tool reports "locked" honestly, nothing attempts
// a network call. Tokens stored locally at tools/zoom-mcp/tokens.json
// (0600), never in the graph, never synced. The host meters/gates/
// ripples every call through /mcp/call regardless.
//
// Real, documented Zoom OAuth difference from Google (confirmed, not
// guessed): the token exchange uses HTTP Basic auth
// (`Authorization: Basic base64(client_id:client_secret)`), not a
// form-body client_id/secret like Google's — see tokenRequest() below.
// Authorize/token endpoints: zoom.us/oauth/authorize, zoom.us/oauth/token.
// API base: api.zoom.us/v2. PKCE kept for parity with gcal-mcp (Zoom
// supports it for public OAuth clients).
//
// HONEST GAP, not guessed: the exact required OAuth scope string(s) for
// a Zoom Server-to-Server or User-level OAuth app (e.g.
// "meeting:write:meeting meeting:read:meeting" on Zoom's newer granular
// scopes, or legacy "meeting:write meeting:read") depend on which app
// type is registered in the Zoom Marketplace — SCOPES below is a
// reasonable real default, but must be verified against the actual
// registered app's granted scopes before first real use, not assumed.
//
// Tools (named in UNIVERSAL_ORDER_GUIDE.md §3: zoom_list_meetings,
// zoom_create_meeting, zoom_join_url — plus the same real auth/status/
// sync scaffolding gcal-mcp needed for an honest locked-by-default
// implementation, not individually named there but required by the
// guide's own general connector rule: "any external API: a connector MCP
// with OAuth, idempotent external_id-keyed sync into task records"):
//   zoom_status                      → locked/authed + account info
//   zoom_auth_start {}               → print the consent URL
//   zoom_auth_code {input:{code}}    → exchange the code for tokens
//   zoom_list_meetings {input:{type?}}         → GET /users/me/meetings
//   zoom_create_meeting {input:{topic,start_time,duration,timezone?}}
//   zoom_join_url {input:{meeting_id}}         → GET /meetings/{id}
//   zoom_sync_to_order {input:{project_id?}}   → same idempotent-per-pass
//        external_id-keyed sync into task records gcal_sync_to_order uses
//        (kind:"meeting", meeting_url = the real join_url)
//
// Env: OZONE_ZOOM_PORT (default 3265), OZONE_ZOOM_CLIENT_ID/SECRET,
//      OZONE_HOST (default http://127.0.0.1:50051), OZONE_ZOOM_PROJECT
//      (default 3, same convention as gcal-mcp's OZONE_GCAL_PROJECT).

import { createServer } from "node:http";
import fs from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";
import crypto from "node:crypto";

const PORT = Number(process.env.OZONE_ZOOM_PORT ?? 3265);
const CLIENT_ID = process.env.OZONE_ZOOM_CLIENT_ID ?? "";
const CLIENT_SECRET = process.env.OZONE_ZOOM_CLIENT_SECRET ?? "";
const HOST = process.env.OZONE_HOST ?? "http://127.0.0.1:50051";
const PROJECT_ID = Number(process.env.OZONE_ZOOM_PROJECT ?? 3);
const TOKEN_FILE = path.join(path.dirname(fileURLToPath(import.meta.url)), "tokens.json");
// Real default — verify against the actual registered Zoom app's granted
// scopes before first real use (see the header's HONEST GAP note).
const SCOPES = "meeting:write:meeting meeting:read:meeting user:read:user";
const REDIRECT_URI = "urn:ietf:wg:oauth:2.0:oob";
const AUTH_BASE = "https://zoom.us/oauth";
const API_BASE = "https://api.zoom.us/v2";

function locked() {
  return !(CLIENT_ID && CLIENT_SECRET);
}

let tokens = null;
try { tokens = JSON.parse(fs.readFileSync(TOKEN_FILE, "utf8")); } catch {}

function saveTokens(t) {
  tokens = t;
  fs.writeFileSync(TOKEN_FILE, JSON.stringify(tokens, null, 1), { mode: 0o600 });
}

// Real Zoom-specific difference from gcal-mcp's googleApi(): the token
// endpoint requires HTTP Basic auth with the client credentials, not a
// form-body client_id/secret (confirmed against Zoom's own OAuth docs).
function basicAuthHeader() {
  return "Basic " + Buffer.from(`${CLIENT_ID}:${CLIENT_SECRET}`).toString("base64");
}

async function tokenRequest(params) {
  const res = await fetch(`${AUTH_BASE}/token`, {
    method: "POST",
    headers: {
      "Content-Type": "application/x-www-form-urlencoded",
      Authorization: basicAuthHeader(),
    },
    body: new URLSearchParams(params),
  });
  return res.json();
}

async function zoomApi(endpoint, opts = {}) {
  if (!tokens?.access_token) throw new Error("not authenticated — run zoom_auth_start first");
  const res = await fetch(`${API_BASE}${endpoint}`, {
    ...opts,
    headers: {
      Authorization: `Bearer ${tokens.access_token}`,
      "Content-Type": "application/json",
      ...(opts.headers ?? {}),
    },
  });
  if (res.status === 401 && tokens.refresh_token) {
    const rt = await tokenRequest({ grant_type: "refresh_token", refresh_token: tokens.refresh_token });
    if (rt.access_token) { tokens.access_token = rt.access_token; if (rt.refresh_token) tokens.refresh_token = rt.refresh_token; saveTokens(tokens); }
    return zoomApi(endpoint, opts);
  }
  const body = await res.json().catch(() => ({}));
  if (!res.ok) throw new Error(body.message ?? `HTTP ${res.status}`);
  return body;
}

function reply(res, code, body) {
  res.writeHead(code, { "Content-Type": "application/json" });
  res.end(JSON.stringify(body));
}

// Same real mapping convention as gcal-mcp's eventToTaskInput — a Zoom
// meeting becomes a "meeting" task, meeting_url = the real join_url, no
// foreign schema stored.
function meetingToTaskInput(m) {
  return {
    prompt: m.topic ?? "(untitled meeting)",
    kind: "meeting",
    due_at: m.start_time ? Math.floor(new Date(m.start_time).getTime() / 1000) : null,
    meeting_url: m.join_url ?? null,
    note: m.agenda ?? "",
    source: "zoom",
    external_ref: { provider: "zoom", external_id: String(m.id), last_synced: Math.floor(Date.now() / 1000) },
    project_id: PROJECT_ID,
  };
}

// ── Idempotent order sync (2026-11-01) ───────────────────────────────────
// WHY a local map: the host's /task/list and /order/global do NOT expose a
// task's external_ref (and /task/create did not persist it when this was
// written), so "look up the existing task by external_id" has nothing to
// read. This MCP therefore keeps its own external_id -> task record map
// (order_sync.json beside this file, 0600, never in the graph) and uses the
// host only to create tasks and to mark them failed/cancelled via
// /task/update (the only status writes that route accepts: completed |
// failed | queued). Result: a re-sync NEVER duplicates — unchanged events
// are skipped, changed events supersede (old task marked failed with a
// reason, ONE replacement created), cancelled/removed events mark their
// task failed with a reason. Never deletes history.
const SYNC_FILE = path.join(path.dirname(fileURLToPath(import.meta.url)), "order_sync.json");
let syncMap = {};
try { syncMap = JSON.parse(fs.readFileSync(SYNC_FILE, "utf8")); } catch {}
function saveSyncMap() {
  try { fs.writeFileSync(SYNC_FILE, JSON.stringify(syncMap, null, 1), { mode: 0o600 }); } catch {}
}
function fingerprint(ti) {
  return crypto.createHash("sha1")
    .update(JSON.stringify([ti.prompt, ti.due_at, ti.meeting_url, ti.note]))
    .digest("hex");
}
function internalHeaders() {
  const h = { "Content-Type": "application/json" };
  let t = process.env.OZONE_INTERNAL_TOKEN || "";
  if (!t) {
    const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
    for (const f of [
      process.env.OZONE_INTERNAL_TOKEN_FILE,
      process.env.OZONE_DATA_DIR && path.join(process.env.OZONE_DATA_DIR, ".internal_token"),
      path.join(repo, "target", "release", "zsei_data", ".internal_token"),
      path.join(repo, "zsei_data", ".internal_token"),
      "/opt/ozone-studio/zsei_data/.internal_token",
    ]) {
      if (!f) continue;
      try { t = fs.readFileSync(f, "utf8").trim(); if (t) break; } catch {}
    }
  }
  if (t) h["x-oz-internal"] = t;
  return h;
}
async function hostPost(route, body) {
  const r = await fetch(`${HOST}${route}`, { method: "POST", headers: internalHeaders(), body: JSON.stringify(body) });
  let j = null;
  try { j = await r.json(); } catch {}
  return { ok: r.ok && j?.success !== false, status: r.status, body: j };
}
async function createOrderTask(ti, sessionToken) {
  // session_token is a REQUIRED String on /task/create — null made the old
  // sync fail deserialisation silently; "" is what local MCP callers send.
  const r = await hostPost("/task/create", { ...ti, created_by: ti.source, session_token: sessionToken ?? "" });
  return r.ok ? r.body?.task_id ?? null : null;
}
async function retireOrderTask(taskId, reason) {
  const r = await hostPost("/task/update", { task_id: taskId, status: "failed", error: reason, agent: "order-sync" });
  return r.ok;
}
// items: [{ external_id, ti, cancelled }]; window: {min,max} unix secs or null.
async function syncToOrder(provider, items, window, sessionToken) {
  const out = { created: 0, updated: 0, unchanged: 0, cancelled: 0, failed: 0, removed: 0 };
  const seen = new Set();
  for (const it of items) {
    const key = `${provider}:${it.external_id}`;
    seen.add(key);
    const known = syncMap[key];
    if (it.cancelled) {
      if (known && known.status === "active") {
        if (await retireOrderTask(known.task_id, `cancelled in ${provider}`)) { known.status = "cancelled"; out.cancelled++; } else out.failed++;
      }
      continue;
    }
    const fp = fingerprint(it.ti);
    if (known && known.status === "active" && known.fp === fp) { out.unchanged++; continue; }
    if (known && known.status === "active") {
      // Changed upstream: supersede (fields cannot be patched through /task/update).
      await retireOrderTask(known.task_id, `superseded: event changed in ${provider}`);
      out.updated++;
    }
    const id = await createOrderTask(it.ti, sessionToken);
    if (id == null) { out.failed++; if (known) known.status = "retired-unreplaced"; continue; }
    syncMap[key] = { task_id: id, fp, due_at: it.ti.due_at ?? null, status: "active", last_synced: Math.floor(Date.now() / 1000) };
    if (!known || known.status !== "active") out.created++;
  }
  // Events that disappeared from the window since the last pass.
  for (const [key, v] of Object.entries(syncMap)) {
    if (!key.startsWith(`${provider}:`) || v.status !== "active" || seen.has(key)) continue;
    if (window && v.due_at != null && (v.due_at < window.min || v.due_at > window.max)) continue;
    if (await retireOrderTask(v.task_id, `removed from ${provider}`)) { v.status = "cancelled"; out.removed++; } else out.failed++;
  }
  saveSyncMap();
  return out;
}

const pendingAuth = { verifier: null };

import { guardOk } from "../shared/guard.mjs";
const server = createServer((req, res) => {
  if (!guardOk(req, res)) return; // loopback guard (tools/shared/guard.mjs)
  if (req.method !== "POST" || !(req.url ?? "").startsWith("/call")) {
    reply(res, 404, { error: "POST /call only" });
    return;
  }
  const chunks = [];
  req.on("data", (c) => chunks.push(c));
  req.on("end", async () => {
    let tool = "";
    let input = {};
    try {
      const body = JSON.parse(Buffer.concat(chunks).toString("utf8") || "{}");
      tool = String(body?.tool ?? "");
      input = body?.input ?? {};
    } catch {
      reply(res, 400, { success: false, error: "invalid JSON body" });
      return;
    }
    try {
      if (locked()) {
        reply(res, 200, { success: false, error: "LOCKED — set OZONE_ZOOM_CLIENT_ID + OZONE_ZOOM_CLIENT_SECRET (a Zoom OAuth app's credentials) to enable this connector" });
        return;
      }
      if (tool === "zoom_status") {
        reply(res, 200, { success: true, output: {
          locked: false,
          authenticated: Boolean(tokens?.access_token),
          has_refresh: Boolean(tokens?.refresh_token),
        }});
      } else if (tool === "zoom_auth_start") {
        const verifier = crypto.randomBytes(32).toString("base64url");
        pendingAuth.verifier = verifier;
        const challenge = crypto.createHash("sha256").update(verifier).digest("base64url");
        const url = `${AUTH_BASE}/authorize?` + new URLSearchParams({
          client_id: CLIENT_ID, redirect_uri: REDIRECT_URI,
          response_type: "code", scope: SCOPES,
          code_challenge: challenge, code_challenge_method: "S256",
        });
        reply(res, 200, { success: true, output: { open_this_url: url, then: "call zoom_auth_code with the pasted code" } });
      } else if (tool === "zoom_auth_code") {
        const code = String(input.code ?? "");
        if (!code || !pendingAuth.verifier) throw new Error("run zoom_auth_start first");
        const t = await tokenRequest({
          grant_type: "authorization_code", code,
          redirect_uri: REDIRECT_URI, code_verifier: pendingAuth.verifier,
        });
        if (!t.access_token) throw new Error(`token exchange failed: ${t.reason ?? t.error ?? "unknown"}`);
        saveTokens(t);
        reply(res, 200, { success: true, output: { authenticated: true } });
      } else if (tool === "zoom_list_meetings") {
        const type = input.type ?? "upcoming";
        const meetings = await zoomApi(`/users/me/meetings?type=${encodeURIComponent(type)}&page_size=50`);
        reply(res, 200, { success: true, output: { meetings: meetings.meetings ?? [] } });
      } else if (tool === "zoom_create_meeting") {
        if (!input.topic || !input.start_time) throw new Error("topic and start_time are both required");
        const m = await zoomApi(`/users/me/meetings`, {
          method: "POST",
          body: JSON.stringify({
            topic: input.topic,
            type: 2, // scheduled meeting — a real, fixed start_time
            start_time: input.start_time,
            duration: input.duration ?? 30,
            timezone: input.timezone ?? "UTC",
            agenda: input.agenda ?? "",
          }),
        });
        reply(res, 200, { success: true, output: { meeting_id: m.id, join_url: m.join_url, start_url: m.start_url } });
      } else if (tool === "zoom_join_url") {
        if (!input.meeting_id) throw new Error("meeting_id required");
        const m = await zoomApi(`/meetings/${encodeURIComponent(input.meeting_id)}`);
        reply(res, 200, { success: true, output: { meeting_id: m.id, join_url: m.join_url, topic: m.topic } });
      } else if (tool === "zoom_sync_to_order") {
        // Same idempotent external_id diff sync as gcal_sync_to_order (syncToOrder).
        const meetings = await zoomApi(`/users/me/meetings?type=upcoming&page_size=50`);
        const items = (meetings.meetings ?? []).map((m) => ({
          external_id: String(m.id), cancelled: false, ti: meetingToTaskInput(m),
        }));
        // "upcoming" listing has no fixed window: only retire tasks whose
        // recorded due time is still in the future and no longer listed.
        const win = { min: Math.floor(Date.now() / 1000), max: Number.MAX_SAFE_INTEGER };
        const r = await syncToOrder("zoom", items, win, input.session_token);
        reply(res, 200, { success: true, output: {
          meetings_seen: items.length,
          tasks_created: r.created, tasks_superseded: r.updated, tasks_unchanged: r.unchanged,
          tasks_cancelled: r.cancelled + r.removed, tasks_failed: r.failed,
          note: "Idempotent by external_id via order_sync.json: re-syncs never duplicate. Changed meetings supersede (old task marked failed with a reason, one replacement created); upcoming meetings that disappeared are marked failed with a reason. tasks_failed counts host rejections (see host log).",
        }});
      } else {
        reply(res, 200, { success: false, error: `unknown zoom tool '${tool}' (zoom_status, zoom_auth_start, zoom_auth_code, zoom_list_meetings, zoom_create_meeting, zoom_join_url, zoom_sync_to_order)` });
      }
    } catch (e) {
      reply(res, 200, { success: false, error: e instanceof Error ? e.message : String(e) });
    }
  });
});

server.on("error", (err) => {
  if (err.code === "EADDRINUSE") { console.error(`[zoom-mcp] :${PORT} in use — skipping`); return; }
  console.error("[zoom-mcp] server error:", err);
});

server.listen(PORT, "127.0.0.1", () => {
  console.error(`[zoom-mcp] /call on 127.0.0.1:${PORT} | ${locked() ? "LOCKED (no OAuth credentials)" : "credentials present"}`);
});
