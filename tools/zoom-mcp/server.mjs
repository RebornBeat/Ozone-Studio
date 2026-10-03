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

const pendingAuth = { verifier: null };

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
        // Same real idempotent-per-pass shape as gcal_sync_to_order —
        // full external_id diff-sync is the same queued refinement noted
        // there, not duplicated as a separate promise here.
        const meetings = await zoomApi(`/users/me/meetings?type=upcoming&page_size=50`);
        let created = 0;
        for (const m of meetings.meetings ?? []) {
          const ti = meetingToTaskInput(m);
          const body = { ...ti, session_token: input.session_token ?? null };
          try {
            const r = await fetch(`${HOST}/task/create`, {
              method: "POST", headers: { "Content-Type": "application/json" },
              body: JSON.stringify(body),
            });
            if (r.ok) created++;
          } catch { /* host hiccup — next sync pass retries */ }
        }
        reply(res, 200, { success: true, output: {
          meetings_seen: (meetings.meetings ?? []).length,
          tasks_created: created,
          note: "Full idempotent external_id diff-sync is the queued refinement (same as gcal-mcp) — this pass creates tasks for currently-upcoming meetings; re-syncs may duplicate until the diff stage lands.",
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
