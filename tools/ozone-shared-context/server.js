#!/usr/bin/env node
/**
 * Ozone-Studio Shared Context — MCP server (stdio, zero dependencies).
 *
 * Three-way coordination between ZCode, Claude Code, and Ozone-Studio:
 *   • presence   — who is working, on what, right now
 *   • claims     — advisory file locks so two agents never overwrite
 *   • notes      — handoffs / findings / decisions, append-only
 *   • cc_sessions— read-only view of Claude Code session logs
 *   • summary    — one call: everything another agent needs to continue work
 *   • search     — non-blocking search_request/search_result bridge: any
 *                   agent can ask for a real web search, fulfilled by a live
 *                   Claude Code session's own native search tool (ZCode and
 *                   pipeline 56 have no Brave API key on this host)
 *
 * Canonical state lives in `.ozone-context/` inside the working directory
 * (git-visible to every agent). Events mirror into the Ozone-Studio host's
 * monitor feed (OZONE_HOST, default http://127.0.0.1:50051) so the graph
 * dashboard sees agent coordination live. ZSEI container mirroring (graph
 * relationships, file links) layers on top of the same events next.
 *
 * Protocol: MCP 2024-11-05 over stdio, newline-delimited JSON-RPC 2.0.
 * Run:  node server.js
 * Env:  OZONE_HOST, OZONE_AGENT_NAME, OZONE_CC_PROJECTS (Claude projects dir)
 */

const fs = require("fs");
const path = require("path");
const os = require("os");

const OZONE_HOST = process.env.OZONE_HOST || "http://127.0.0.1:50051";
const STATE_DIR = process.env.OZONE_CONTEXT_DIR || path.join(process.cwd(), ".ozone-context");
const CC_PROJECTS = process.env.OZONE_CC_PROJECTS || path.join(os.homedir(), ".claude", "projects");
const PRESENCE_TTL_MS = 5 * 60 * 1000; // sessions expire from "live" after 5 min idle

// ── storage ───────────────────────────────────────────────────────────────

function ensureStateDir() {
  fs.mkdirSync(STATE_DIR, { recursive: true });
}

function loadState() {
  ensureStateDir();
  const file = path.join(STATE_DIR, "state.json");
  try {
    return JSON.parse(fs.readFileSync(file, "utf8"));
  } catch {
    return { sessions: {}, claims: {} };
  }
}

function saveState(state) {
  ensureStateDir();
  fs.writeFileSync(
    path.join(STATE_DIR, "state.json"),
    JSON.stringify(state, null, 2),
  );
}

function appendNote(note) {
  ensureStateDir();
  fs.appendFileSync(path.join(STATE_DIR, "notes.jsonl"), JSON.stringify(note) + "\n");
}

function readNotes(limit = 30, kind = null) {
  ensureStateDir();
  const file = path.join(STATE_DIR, "notes.jsonl");
  let lines = [];
  try {
    lines = fs.readFileSync(file, "utf8").split("\n").filter(Boolean);
  } catch {
    return [];
  }
  let notes = lines.map((l) => {
    try { return JSON.parse(l); } catch { return null; }
  }).filter(Boolean);
  if (kind) notes = notes.filter((n) => n.kind === kind);
  return notes.slice(-limit).reverse();
}

// ── ozone host mirroring (best-effort, never blocking) ────────────────────

function pushActivity(kind, level, source, message) {
  fetch(`${OZONE_HOST}/monitor/activity`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ kind, level, source, message }),
  }).catch(() => {}); // host down → local state still works
}

// Graph mirroring (task 42): coordination events become real ZSEI
// containers under /SharedContext via the host's /context/mirror.
// Fire-and-forget — graph state lags local state gracefully when offline.
function mirrorContext(req) {
  fetch(`${OZONE_HOST}/context/mirror`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(req),
  }).catch(() => {});
}

const AGENT_NAME =
  process.env.OZONE_AGENT_NAME ||
  process.env.USER ||
  "unknown-agent"; // attribution matters: never silently anonymous if avoidable

// ── SCOPE (declared when this server/session starts, like the UI declares
// workspace on run): the workspace/project this coordination binding belongs
// to. Events default to workspace scope; explicit overrides per event;
// promotion to global is always deliberate.
const SCOPE = process.env.OZONE_SCOPE || "workspace"; // global | workspace | project
const WORKSPACE_ID = process.env.OZONE_WORKSPACE_ID || null;
const PROJECT_ID = process.env.OZONE_PROJECT_ID || null;

function eventScope(override) {
  const o = override || {};
  const scope = o.scope || SCOPE;
  return {
    scope,
    workspace_id:
      o.workspace_id !== undefined ? o.workspace_id : WORKSPACE_ID ? Number(WORKSPACE_ID) : null,
    project_id:
      o.project_id !== undefined ? o.project_id : PROJECT_ID ? Number(PROJECT_ID) : null,
  };
}

// ── host session (Ed25519 device auth — needed for /task/* routes) ────────

const crypto = require("crypto");
let cachedToken = null;

function deviceKeypair() {
  const keyFile = path.join(STATE_DIR, "device-key.pem");
  ensureStateDir();
  let pem;
  if (fs.existsSync(keyFile)) {
    pem = fs.readFileSync(keyFile, "utf8");
  } else {
    const kp = crypto.generateKeyPairSync("ed25519");
    pem = kp.privateKey.export({ type: "pkcs8", format: "pem" });
    fs.writeFileSync(keyFile, pem);
  }
  const privateKey = crypto.createPrivateKey(pem);
  const spki = crypto.createPublicKey(privateKey).export({ type: "spki", format: "der" });
  return { privateKey, rawPub: spki.subarray(spki.length - 32).toString("hex") };
}

async function hostToken() {
  if (cachedToken) return cachedToken;
  const post = (p, b) =>
    fetch(`${OZONE_HOST}${p}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(b),
    }).then((r) => r.json());
  const { privateKey, rawPub } = deviceKeypair();
  const ch = await post("/auth/challenge", { public_key: rawPub });
  if (!ch?.challenge) throw new Error("no challenge from host");
  const signature = crypto.sign(null, Buffer.from(ch.challenge, "hex"), privateKey).toString("hex");
  const auth = await post("/auth/authenticate", { public_key: rawPub, signature });
  if (!auth?.success || !auth.session_token) throw new Error("host auth failed: " + (auth?.error || "?"));
  cachedToken = auth.session_token;
  return cachedToken;
}

// ── MCP usage gate — the budget lives on the host, enforced per call ──────

async function usageGate(tool) {
  try {
    const res = await fetch(`${OZONE_HOST}/mcp/usage`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ agent: AGENT_NAME, tool }),
    });
    if (res.ok) {
      const j = await res.json();
      if (j.allowed === false) {
        return { blocked: true, total: j.total_today, limit: j.daily_limit };
      }
      return { blocked: false };
    }
  } catch {}
  return { blocked: false, offline: true }; // host unreachable → local mode
}

// ── core operations ───────────────────────────────────────────────────────

function isLive(lastSeen) {
  return Date.now() - lastSeen < PRESENCE_TTL_MS;
}

function presenceHeartbeat({ agent, role, current_files, task }) {
  const state = loadState();
  const isNew = !state.sessions[agent];
  state.sessions[agent] = {
    role: role || "agent",
    current_files: current_files || [],
    task: task || "",
    last_seen: Date.now(),
  };
  // Drop expired claims held by agents that went silent.
  for (const [file, claim] of Object.entries(state.claims)) {
    if (!state.sessions[claim.agent] || !isLive(state.sessions[claim.agent].last_seen)) {
      delete state.claims[file];
    }
  }
  saveState(state);
  if (isNew) {
    pushActivity("agent", "ok", agent, `${agent} joined shared context (role: ${role || "agent"})`);
  }
  return { ok: true, agent, live: true };
}

function presenceList() {
  const state = loadState();
  const live = [];
  for (const [agent, s] of Object.entries(state.sessions)) {
    if (isLive(s.last_seen)) {
      live.push({ agent, role: s.role, current_files: s.current_files, task: s.task, last_seen_age_s: Math.round((Date.now() - s.last_seen) / 1000) });
    }
  }
  return { live, ttl_seconds: PRESENCE_TTL_MS / 1000 };
}

function fileClaim({ agent, files, reason }) {
  const state = loadState();
  const conflicts = [];
  for (const f of files || []) {
    const existing = state.claims[f];
    if (existing && existing.agent !== agent && isLive((state.sessions[existing.agent] || {}).last_seen || 0)) {
      conflicts.push({ file: f, held_by: existing.agent, reason: existing.reason });
    }
  }
  if (conflicts.length > 0) {
    return { ok: false, conflicts, message: "Files claimed by another live agent — coordinate before editing (see their presence task / latest notes)." };
  }
  for (const f of files || []) {
    state.claims[f] = { agent, reason: reason || "", at: Date.now() };
  }
  saveState(state);
  if ((files || []).length > 0) {
    pushActivity("bridge", "info", agent, `${agent} claimed ${files.length} file(s): ${reason || "(no reason given)"}`);
    // Mirror each claim into the coordination graph (idempotent per path).
    for (const f of files || []) {
      mirrorContext({ kind: "claim", agent, title: `claim: ${f}`, body: reason || "", files: [f], ...eventScope({ scope: "workspace" }) });
    }
  }
  return { ok: true, claimed: files || [] };
}

function fileRelease({ agent, files }) {
  const state = loadState();
  const released = [];
  for (const f of files || []) {
    if (state.claims[f] && state.claims[f].agent === agent) {
      delete state.claims[f];
      released.push(f);
    }
  }
  saveState(state);
  return { ok: true, released };
}

function fileClaims() {
  const state = loadState();
  const claims = [];
  for (const [file, c] of Object.entries(state.claims)) {
    claims.push({ file, agent: c.agent, reason: c.reason, age_min: Math.round((Date.now() - c.at) / 60000) });
  }
  return { claims };
}

function noteAdd({ agent, kind, title, body, files, scope, workspace_id, project_id }) {
  const note = {
    id: Date.now().toString(36) + Math.random().toString(36).slice(2, 6),
    agent, kind, title, body: body || "",
    files: files || [],
    scope: scope || SCOPE,
    at: Date.now(),
  };
  appendNote(note);
  pushActivity("log", "info", agent, `[${kind}] ${title}`);
  mirrorContext({
    kind: kind === "claim" ? "claim" : kind,
    agent,
    title,
    body: body || "",
    files: files || [],
    detail: { note_id: note.id },
    ...eventScope({ scope, workspace_id, project_id }),
  });
  return { ok: true, id: note.id };
}

// ── web search bridge — Claude Code has a real, working native web search
// tool; ZCode/pipeline 56 do not (pipeline 56 needs a Brave API key this
// host doesn't have). Rather than requiring that key, route search needs
// through whichever Claude Code session is live: any agent posts a query
// here (non-blocking — returns an id immediately), Claude Code answers it
// with REAL results from its own tool when it next checks in, the
// requester polls by id. This is genuinely async — there is no guarantee
// a Claude Code session is live/watching at request time, so a requester
// should poll rather than assume an immediate answer.
//
// State: `.ozone-context/search_requests.jsonl`, append-only, two record
// kinds distinguished by `status`:
//   requests: {id, requester, query, status:"pending", at}
//   answers:  overwrite in place — same id, status:"answered", results, answered_by, answered_at}
// Overwriting (not appending a second record) keeps "poll by id" a single
// lookup rather than requiring the caller to reconcile a request+answer pair.
const SEARCH_FILE = path.join(STATE_DIR, "search_requests.jsonl");

function loadSearchRequests() {
  ensureStateDir();
  try {
    return fs.readFileSync(SEARCH_FILE, "utf8").split("\n").filter(Boolean).map((l) => {
      try { return JSON.parse(l); } catch { return null; }
    }).filter(Boolean);
  } catch {
    return [];
  }
}

function saveSearchRequests(list) {
  ensureStateDir();
  fs.writeFileSync(SEARCH_FILE, list.map((r) => JSON.stringify(r)).join("\n") + (list.length ? "\n" : ""));
}

function searchRequest({ agent, query }) {
  if (!query || !query.trim()) {
    return { ok: false, error: "query is required" };
  }
  const list = loadSearchRequests();
  const id = "sr-" + Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
  list.push({ id, requester: agent, query: query.trim(), status: "pending", at: Date.now() });
  saveSearchRequests(list);
  pushActivity("job", "info", agent, `search request ${id}: "${query.trim().slice(0, 80)}"`);
  return { ok: true, id, note: "Non-blocking — poll search_result with this id. A Claude Code session must be live and check pending requests to answer it; there is no guaranteed latency." };
}

function searchPending({ limit = 20 } = {}) {
  const list = loadSearchRequests();
  return { pending: list.filter((r) => r.status === "pending").slice(-limit) };
}

function searchResult({ id }) {
  if (!id) return { ok: false, error: "id is required" };
  const list = loadSearchRequests();
  const rec = list.find((r) => r.id === id);
  if (!rec) return { ok: false, error: `no request with id ${id}` };
  if (rec.status === "pending") return { ok: true, status: "pending", query: rec.query, at: rec.at };
  return { ok: true, status: "answered", query: rec.query, results: rec.results, answered_by: rec.answered_by, answered_at: rec.answered_at };
}

// answer_search: how Claude Code (the only agent with a real search tool
// today) fulfills a pending request after actually performing the search
// with its own native tool. `results` must be real — this bridge has no
// way to verify that, same trust boundary as every other note in this
// file; don't fabricate results to close out a request faster.
function answerSearch({ agent, id, results }) {
  const list = loadSearchRequests();
  const rec = list.find((r) => r.id === id);
  if (!rec) return { ok: false, error: `no request with id ${id}` };
  rec.status = "answered";
  rec.results = results || [];
  rec.answered_by = agent;
  rec.answered_at = Date.now();
  saveSearchRequests(list);
  pushActivity("job", "ok", agent, `answered search ${id} (${(results || []).length} result(s))`);
  return { ok: true, id };
}

function ccSessions({ limit = 8 } = {}) {
  // Claude Code munges project paths: /home/x/Proj → -home-x-Proj
  const munge = process.cwd().replace(/\//g, "-");
  const dirs = [path.join(CC_PROJECTS, munge), CC_PROJECTS];
  const sessions = [];
  for (const dir of dirs) {
    let entries = [];
    try { entries = fs.readdirSync(dir); } catch { continue; }
    for (const e of entries) {
      if (!e.endsWith(".jsonl")) continue;
      const full = path.join(dir, e);
      try {
        const st = fs.statSync(full);
        // Last meaningful line: tail cheaply (last 4KB).
        const fd = fs.openSync(full, "r");
        const tail = Buffer.alloc(Math.min(4096, st.size));
        fs.readSync(fd, tail, 0, tail.length, Math.max(0, st.size - tail.length));
        fs.closeSync(fd);
        const lines = tail.toString("utf8").split("\n").filter(Boolean);
        let lastType = "", lastPreview = "";
        for (let i = lines.length - 1; i >= 0; i--) {
          try {
            const j = JSON.parse(lines[i]);
            if (j.type && !lastType) {
              lastType = j.type;
              const c = j.message?.content;
              lastPreview = typeof c === "string" ? c.slice(0, 140) : Array.isArray(c) ? (c.find((p) => p.text)?.text ?? "").slice(0, 140) : "";
              break;
            }
          } catch {}
        }
        sessions.push({ session: e.replace(/\.jsonl$/, ""), age_min: Math.round((Date.now() - st.mtimeMs) / 60000), last_type: lastType, last_preview: lastPreview });
      } catch {}
    }
    if (sessions.length > 0) break; // project-scoped dir matched — enough
  }
  sessions.sort((a, b) => a.age_min - b.age_min);
  return { sessions: sessions.slice(0, limit), note: "read-only view of Claude Code session logs" };
}

function contextSummary() {
  return {
    agent: AGENT_NAME,
    scope_binding: eventScope({}),
    presence: presenceList(),
    claims: fileClaims().claims,
    latest_notes: readNotes(8),
    cc_sessions: ccSessions({ limit: 5 }).sessions,
    state_dir: STATE_DIR,
    ozone_host: OZONE_HOST,
  };
}

// ── task routing — coordination work becomes real Ozone-Studio tasks ─────

async function taskCreate({ title, description, assignee, priority }) {
  const token = await hostToken();
  const res = await fetch(`${OZONE_HOST}/task/create`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      prompt: description ? `${title}\n\n${description}` : title,
      assignee: assignee || null,
      created_by: AGENT_NAME,
      priority: priority || null,
      session_token: token,
    }),
  });
  const out = await res.json();
  if (out.success) {
    pushActivity("job", "info", AGENT_NAME, `routed task ${out.task_id} → ${assignee || "any agent"}: ${title}`);
  }
  return out;
}

async function tasksList({ limit = 20 } = {}) {
  const token = await hostToken();
  const res = await fetch(`${OZONE_HOST}/task/list`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ session_token: token, limit }),
  });
  return res.json();
}

async function mcpUsage() {
  const res = await fetch(`${OZONE_HOST}/mcp/usage?agent=${encodeURIComponent(AGENT_NAME)}`);
  return res.json();
}

// ── MCP plumbing ──────────────────────────────────────────────────────────

const TOOLS = [
  { name: "presence_heartbeat", description: "Announce you are working: agent name, role, current files, current task. Call at start and whenever focus changes; keeps you 'live' for 5 min.", inputSchema: { type: "object", properties: { agent: { type: "string" }, role: { type: "string", description: "agent | model | reviewer | ..." }, current_files: { type: "array", items: { type: "string" } }, task: { type: "string" } }, required: ["agent"] } },
  { name: "presence_list", description: "List all LIVE agents (last 5 min) with their current files and tasks.", inputSchema: { type: "object", properties: {} } },
  { name: "file_claim", description: "Advisory claim on files you are editing — refuses if another LIVE agent holds a claim (the no-overwrites protocol). Release when done.", inputSchema: { type: "object", properties: { agent: { type: "string" }, files: { type: "array", items: { type: "string" } }, reason: { type: "string" } }, required: ["agent", "files"] } },
  { name: "file_release", description: "Release your claims when done editing.", inputSchema: { type: "object", properties: { agent: { type: "string" }, files: { type: "array", items: { type: "string" } } }, required: ["agent", "files"] } },
  { name: "file_claims", description: "List all active file claims.", inputSchema: { type: "object", properties: {} } },
  { name: "note_add", description: "Append a shared note: kind = handoff | finding | decision. Handoffs tell the next agent how to continue. SCOPE: defaults to this session's declared binding (env OZONE_SCOPE/OZONE_WORKSPACE_ID/OZONE_PROJECT_ID, default workspace); pass scope=\"global\" only for host-wide or cross-workspace findings — cross-workspace visibility is explicit, never silent.", inputSchema: { type: "object", properties: { agent: { type: "string" }, kind: { type: "string", enum: ["handoff", "finding", "decision"] }, title: { type: "string" }, body: { type: "string" }, files: { type: "array", items: { type: "string" } }, scope: { type: "string", enum: ["global", "workspace", "project"], description: "default: session binding (workspace)" }, workspace_id: { type: "number" }, project_id: { type: "number" } }, required: ["agent", "kind", "title"] } },
  { name: "notes_list", description: "Read recent shared notes (newest first).", inputSchema: { type: "object", properties: { limit: { type: "number" }, kind: { type: "string" } } } },
  { name: "cc_sessions", description: "Read-only view of recent Claude Code sessions for this project (last activity + preview).", inputSchema: { type: "object", properties: { limit: { type: "number" } } } },
  { name: "context_summary", description: "ONE CALL before starting work: live agents, file claims, latest notes/handoffs, recent CC sessions.", inputSchema: { type: "object", properties: {} } },
  { name: "task_create", description: "Route work through Ozone-Studio: creates a REAL queued task (tracked, listed, monitored) optionally assigned to an agent. Use for handoffs that are work, not just notes.", inputSchema: { type: "object", properties: { title: { type: "string" }, description: { type: "string" }, assignee: { type: "string", description: "zcode | claude-code | …" }, priority: { type: "string", enum: ["low", "normal", "high"] } }, required: ["title"] } },
  { name: "tasks_list", description: "List Ozone-Studio tasks (includes coordination tasks routed to agents).", inputSchema: { type: "object", properties: { limit: { type: "number" } } } },
  { name: "mcp_usage", description: "Your own MCP usage: today's calls, daily limit, remaining, per-tool breakdown.", inputSchema: { type: "object", properties: {} } },
  { name: "search_request", description: "Ask for a real web search — non-blocking, returns an id immediately. Answered by whichever Claude Code session is live and checking pending requests (Claude Code has a real native search tool; ZCode/pipeline 56 do not, no Brave API key on this host). Poll search_result with the id; there is no guaranteed latency.", inputSchema: { type: "object", properties: { agent: { type: "string" }, query: { type: "string" } }, required: ["agent", "query"] } },
  { name: "search_pending", description: "List pending (unanswered) search requests — call this if you are a Claude Code session willing to answer some with your real search tool.", inputSchema: { type: "object", properties: { limit: { type: "number" } } } },
  { name: "search_result", description: "Poll a search request by id — status:pending or status:answered with real results.", inputSchema: { type: "object", properties: { id: { type: "string" } }, required: ["id"] } },
  { name: "answer_search", description: "Fulfill a pending search request with real results you actually retrieved via your own search tool — never fabricate results to close a request.", inputSchema: { type: "object", properties: { agent: { type: "string" }, id: { type: "string" }, results: { type: "array", items: { type: "object", properties: { title: { type: "string" }, url: { type: "string" }, snippet: { type: "string" } } } } }, required: ["agent", "id", "results"] } },
];

function callTool(name, args) {
  switch (name) {
    case "presence_heartbeat": return presenceHeartbeat(args);
    case "presence_list": return presenceList();
    case "file_claim": return fileClaim(args);
    case "file_release": return fileRelease(args);
    case "file_claims": return fileClaims();
    case "note_add": return noteAdd(args);
    case "notes_list": return readNotes(args?.limit ?? 30, args?.kind ?? null);
    case "cc_sessions": return ccSessions(args);
    case "context_summary": return contextSummary();
    case "task_create": return taskCreate(args);
    case "tasks_list": return tasksList(args);
    case "mcp_usage": return mcpUsage();
    case "search_request": return searchRequest(args);
    case "search_pending": return searchPending(args);
    case "search_result": return searchResult(args);
    case "answer_search": return answerSearch(args);
    default: throw new Error(`unknown tool: ${name}`);
  }
}

function reply(id, result) {
  process.stdout.write(JSON.stringify({ jsonrpc: "2.0", id, result }) + "\n");
}

function replyError(id, code, message) {
  process.stdout.write(JSON.stringify({ jsonrpc: "2.0", id, error: { code, message } }) + "\n");
}

let buffer = "";
process.stdin.setEncoding("utf8");
process.stdin.on("data", (chunk) => {
  buffer += chunk;
  let idx;
  while ((idx = buffer.indexOf("\n")) >= 0) {
    const line = buffer.slice(0, idx).trim();
    buffer = buffer.slice(idx + 1);
    if (!line) continue;
    let msg;
    try { msg = JSON.parse(line); } catch { continue; }
    handleMessage(msg).catch((e) => {
      if (msg.id !== undefined) {
        replyError(msg.id, -32603, `internal error: ${e.message}`);
      }
    });
  }
});

async function handleMessage(msg) {
  const { id, method, params } = msg;
  if (method === "initialize") {
    reply(id, {
      protocolVersion: params?.protocolVersion || "2024-11-05",
      capabilities: { tools: {} },
      serverInfo: { name: "ozone-shared-context", version: "0.1.0" },
    });
    return;
  }
  if (method === "notifications/initialized" || (method || "").startsWith("notifications/")) {
    return; // notifications get no reply
  }
  if (method === "ping") {
    reply(id, {});
    return;
  }
  if (method === "tools/list") {
    reply(id, { tools: TOOLS });
    return;
  }
  if (method === "tools/call") {
    try {
      const gate = await usageGate(params.name);
      if (gate.blocked) {
        reply(id, {
          content: [{
            type: "text",
            text: `MCP daily usage limit reached (${gate.total}/${gate.limit} calls today for ${AGENT_NAME}). The host refused this call — OZONE_MCP_DAILY_LIMIT on the Ozone-Studio host controls it.`,
          }],
          isError: true,
        });
        return;
      }
      const out = await callTool(params.name, params.arguments || {});
      reply(id, { content: [{ type: "text", text: JSON.stringify(out, null, 2) }] });
    } catch (e) {
      reply(id, { content: [{ type: "text", text: `error: ${e.message}` }], isError: true });
    }
    return;
  }
  if (id !== undefined) {
    replyError(id, -32601, `method not found: ${method}`);
  }
}

process.stderr.write("[ozone-shared-context] ready — state in " + STATE_DIR + "\n");

// ── self-registration (connect-model: the tool announces itself) ──────────
// Re-announces hourly so a host restart self-heals without any agent action.
function selfRegister() {
  fetch(`${OZONE_HOST}/mcp/tools/register`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      name: "ozone-shared-context",
      transport: "stdio",
      endpoint: __filename,
      capabilities: ["presence", "file_claims", "handoffs", "cc_session_logs", "summary", "task_routing", "search_bridge"],
      server_version: "0.3.0",
    }),
  }).catch(() => {});
}
selfRegister();
setInterval(selfRegister, 60 * 60 * 1000);
