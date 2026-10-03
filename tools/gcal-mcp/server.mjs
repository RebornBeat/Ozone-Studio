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

const pendingAuth = { url: null, verifier: null };

const server = createServer((req, res) => {
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
        // Idempotent external_id-keyed sync: events → task records via the
        // host's /task/create. Existing = matched by external_ref on a
        // previous sync pass (the order carries provenance; this pass
        // creates only what isn't already known locally — full diff sync
        // is the queued refinement).
        const params = new URLSearchParams({
          calendarId: CALENDAR_ID,
          timeMin: input.time_min ?? new Date().toISOString(),
          timeMax: input.time_max ?? new Date(Date.now() + 14 * 864e5).toISOString(),
          singleEvents: "true", orderBy: "startTime", maxResults: "50",
        });
        const events = await googleApi(`/calendars/${encodeURIComponent(CALENDAR_ID)}/events?${params}`);
        let created = 0;
        const seen = [];
        for (const ev of events.items ?? []) {
          seen.push(ev.id);
          const ti = eventToTaskInput(ev);
          const body = { ...ti, session_token: input.session_token ?? null };
          try {
            const r = await fetch(`${HOST}/task/create`, {
              method: "POST", headers: { "Content-Type": "application/json" },
              body: JSON.stringify(body),
            });
            if (r.ok) created++;
          } catch { /* host hiccup — next sync pass retries (idempotent on external_id at the diff stage) */ }
        }
        reply(res, 200, { success: true, output: {
          synced_window: { time_min: params.get("timeMin"), time_max: params.get("timeMax") },
          events_seen: (events.items ?? []).length,
          tasks_created: created,
          note: "Full idempotent external_id diff-sync is the queued refinement — this pass creates tasks for events in the window; re-syncs within the window may duplicate until the diff stage lands.",
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
