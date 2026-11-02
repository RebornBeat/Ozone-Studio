// Ozone-Studio Google Calendar MCP — Universal Order Stage 3 connector
// (docs/UNIVERSAL_ORDER_GUIDE.md §3). External events land as TASKS with
// external_ref provenance — the order never stores foreign schemas.
//
// SECURITY / LOCKED-BY-DEFAULT:
// - Requires OZONE_GCAL_CLIENT_ID + OZONE_GCAL_CLIENT_SECRET (Google Cloud
//   OAuth installed-app credentials). Without them: locked — every tool
//   reports "locked" honestly, nothing attempts network calls.
// - OAuth installed-app flow: user opens the printed URL, approves, pastes
//   the redirect code back via `gcal_auth <code>` (handled by the auth
//   tool's poll). Tokens stored locally at tools/gcal-mcp/tokens.json
//   (0600), never in the graph, never synced.
// - The host meters/gates/ripples every call through /mcp/call regardless.
//
// Tools:
//   gcal_status                     → locked/authed/expired + account email
//   gcal_auth_start {}              → print the consent URL
//   gcal_auth_code {input:{code}}   → exchange the code for tokens
//   gcal_list_events {input:{time_min,time_max,max_results?}}
//   gcal_create_event {input:{summary,start,end,description?,attendees?}}
//   gcal_sync_to_order {input:{calendar_id?,time_min?,time_max?,project_id?}}
//        → idempotent external_id-keyed sync: events become/UPDATE task
//          records (kind "meeting", external_ref set) via the host's
//          /task/create + /task/update.
//
// Env: OZONE_GCAL_PORT (default 3240), OZONE_GCAL_CLIENT_ID/SECRET,
//      OZONE_HOST (default http://127.0.0.1:50051), OZONE_GCAL_CALENDAR
//      (default "primary"), OZONE_GCAL_PROJECT (default 3).

import { createServer } from "node:http";
import fs from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";
import crypto from "node:crypto";

const PORT = Number(process.env.OZONE_GCAL_PORT ?? 3241);
const CLIENT_ID = process.env.OZONE_GCAL_CLIENT_ID ?? "";
const CLIENT_SECRET = process.env.OZONE_GCAL_CLIENT_SECRET ?? "";
const HOST = process.env.OZONE_HOST ?? "http://127.0.0.1:50051";
const CALENDAR_ID = process.env.OZONE_GCAL_CALENDAR ?? "primary";
const PROJECT_ID = Number(process.env.OZONE_GCAL_PROJECT ?? 3);
const TOKEN_FILE = path.join(path.dirname(fileURLToPath(import.meta.url)), "tokens.json");
const SCOPES = "https://www.googleapis.com/auth/calendar.events https://www.googleapis.com/auth/calendar.readonly";
const REDIRECT_URI = "urn:ietf:wg:oauth:2.0:oob";

function locked() {
  return !(CLIENT_ID && CLIENT_SECRET);
}

let tokens = null;
try { tokens = JSON.parse(fs.readFileSync(TOKEN_FILE, "utf8")); } catch {}

function saveTokens(t) {
  tokens = t;
  fs.writeFileSync(TOKEN_FILE, JSON.stringify(tokens, null, 1), { mode: 0o600 });
}

async function googleApi(endpoint, opts = {}) {
  if (!tokens?.access_token) throw new Error("not authenticated — run gcal_auth_start first");
  const res = await fetch(`https://www.googleapis.com/calendar/v1${endpoint}`, {
    ...opts,
    headers: {
      Authorization: `Bearer ${tokens.access_token}`,
      "Content-Type": "application/json",
      ...(opts.headers ?? {}),
    },
  });
  if (res.status === 401 && tokens.refresh_token) {
    // refresh once, then retry
    const rr = await fetch("https://oauth2.googleapis.com/token", {
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      body: new URLSearchParams({
        client_id: CLIENT_ID, client_secret: CLIENT_SECRET,
        refresh_token: tokens.refresh_token, grant_type: "refresh_token",
      }),
    });
    const rt = await rr.json();
    if (rt.access_token) { tokens.access_token = rt.access_token; saveTokens(tokens); }
    return googleApi(endpoint, opts);
  }
  const body = await res.json();
  if (!res.ok) throw new Error(body.error?.message ?? `HTTP ${res.status}`);
  return body;
}

function reply(res, code, body) {
  res.writeHead(code, { "Content-Type": "application/json" });
  res.end(JSON.stringify(body));
}

function eventToTaskInput(ev) {
  const start = ev.start?.dateTime ?? ev.start?.date ?? "";
  return {
    prompt: ev.summary ?? "(untitled event)",
    kind: "meeting",
    due_at: Math.floor(new Date(start).getTime() / 1000) || null,
    meeting_url: ev.hangoutLink ?? null,
    note: ev.description ?? "",
    source: "google-calendar",
    external_ref: { provider: "google-calendar", external_id: ev.id, last_synced: Math.floor(Date.now() / 1000) },
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

const pendingAuth = { url: null, verifier: null };

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
        reply(res, 200, { success: false, error: "LOCKED — set OZONE_GCAL_CLIENT_ID + OZONE_GCAL_CLIENT_SECRET (Google Cloud OAuth installed-app credentials) to enable this connector" });
        return;
      }
      if (tool === "gcal_status") {
        reply(res, 200, { success: true, output: {
          locked: false,
          authenticated: Boolean(tokens?.access_token),
          has_refresh: Boolean(tokens?.refresh_token),
          calendar: CALENDAR_ID,
        }});
      } else if (tool === "gcal_auth_start") {
        const verifier = crypto.randomBytes(32).toString("base64url");
        pendingAuth.verifier = verifier;
        const challenge = crypto.createHash("sha256").update(verifier).digest("base64url");
        const url = "https://accounts.google.com/o/oauth2/v2/auth?" + new URLSearchParams({
          client_id: CLIENT_ID, redirect_uri: REDIRECT_URI,
          response_type: "code", scope: SCOPES,
          code_challenge: challenge, code_challenge_method: "S256",
          access_type: "offline", prompt: "consent",
        });
        pendingAuth.url = url;
        reply(res, 200, { success: true, output: { open_this_url: url, then: "call gcal_auth_code with the pasted code" } });
      } else if (tool === "gcal_auth_code") {
        const code = String(input.code ?? "");
        if (!code || !pendingAuth.verifier) throw new Error("run gcal_auth_start first");
        const rr = await fetch("https://oauth2.googleapis.com/token", {
          method: "POST",
          headers: { "Content-Type": "application/x-www-form-urlencoded" },
          body: new URLSearchParams({
            client_id: CLIENT_ID, client_secret: CLIENT_SECRET, code,
            code_verifier: pendingAuth.verifier, redirect_uri: REDIRECT_URI,
            grant_type: "authorization_code",
          }),
        });
        const t = await rr.json();
        if (!t.access_token) throw new Error(`token exchange failed: ${t.error ?? "unknown"}`);
        saveTokens(t);
        reply(res, 200, { success: true, output: { authenticated: true } });
      } else if (tool === "gcal_list_events") {
        const params = new URLSearchParams({
          calendarId: CALENDAR_ID,
          timeMin: input.time_min ?? new Date(Date.now() - 864e5).toISOString(),
          timeMax: input.time_max ?? new Date(Date.now() + 30 * 864e5).toISOString(),
          singleEvents: "true", orderBy: "startTime",
          maxResults: String(input.max_results ?? 25),
        });
        const events = await googleApi(`/calendars/${encodeURIComponent(CALENDAR_ID)}/events?${params}`);
        reply(res, 200, { success: true, output: { events: events.items ?? [] } });
      } else if (tool === "gcal_create_event") {
        const ev = await googleApi(`/calendars/${encodeURIComponent(CALENDAR_ID)}/events`, {
          method: "POST",
          body: JSON.stringify({
            summary: input.summary, description: input.description ?? "",
            start: input.start, end: input.end,
            attendees: input.attendees ?? [],
          }),
        });
        reply(res, 200, { success: true, output: { event_id: ev.id, htmlLink: ev.htmlLink } });
      } else if (tool === "gcal_sync_to_order") {
        // Idempotent external_id-keyed diff sync (see syncToOrder above).
        const params = new URLSearchParams({
          calendarId: CALENDAR_ID,
          timeMin: input.time_min ?? new Date().toISOString(),
          timeMax: input.time_max ?? new Date(Date.now() + 14 * 864e5).toISOString(),
          singleEvents: "true", orderBy: "startTime", maxResults: "50",
        });
        const events = await googleApi(`/calendars/${encodeURIComponent(CALENDAR_ID)}/events?${params}`);
        const items = (events.items ?? []).map((ev) => ({
          external_id: ev.id, cancelled: ev.status === "cancelled", ti: eventToTaskInput(ev),
        }));
        const win = { min: Math.floor(new Date(params.get("timeMin")).getTime() / 1000), max: Math.floor(new Date(params.get("timeMax")).getTime() / 1000) };
        const r = await syncToOrder("google-calendar", items, win, input.session_token);
        reply(res, 200, { success: true, output: {
          synced_window: { time_min: params.get("timeMin"), time_max: params.get("timeMax") },
          events_seen: items.length,
          tasks_created: r.created, tasks_superseded: r.updated, tasks_unchanged: r.unchanged,
          tasks_cancelled: r.cancelled + r.removed, tasks_failed: r.failed,
          note: "Idempotent by external_id via order_sync.json: re-syncs never duplicate. Changed events supersede (old task marked failed with a reason, one replacement created); cancelled/removed events mark their task failed with a reason. tasks_failed counts host rejections (see host log).",
        }});
      } else {
        reply(res, 200, { success: false, error: `unknown gcal tool '${tool}'` });
      }
    } catch (e) {
      reply(res, 200, { success: false, error: e instanceof Error ? e.message : String(e) });
    }
  });
});

server.on("error", (err) => {
  if (err.code === "EADDRINUSE") { console.error(`[gcal-mcp] :${PORT} in use — skipping`); return; }
  console.error("[gcal-mcp] server error:", err);
});

server.listen(PORT, "127.0.0.1", () => {
  console.error(`[gcal-mcp] /call on 127.0.0.1:${PORT} | ${locked() ? "LOCKED (no OAuth credentials)" : "credentials present"}`);
});
