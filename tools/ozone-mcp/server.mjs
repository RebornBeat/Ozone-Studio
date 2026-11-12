#!/usr/bin/env node
/**
 * Ozone-Studio MCP surface (stdio, zero dependencies) — lets Claude Code, ZCode and any MCP client use
 * Ozone-Studio AS A THIRD PARTY: run a walk, link files/folders, read call graphs, work orders, capability
 * gaps, the refinement report, tasks, ZSEI containers and marketplace views.
 *
 * Rules (docs/evidence/mcp-surface/REPORT.md):
 *  - Only the host's public HTTP surface. Never reads the data dir, the code tree or the store directly,
 *    and never uses the host's internal token (that is the same-user shortcut the shared-context server has).
 *  - Session auth = the same ed25519 device-key challenge as tools/ozone-cli and tools/e2e. The key and the
 *    session token are never printed, logged or returned (errors are scrubbed).
 *  - Every call carries an agent identity (`agent` arg, else OZONE_AGENT_NAME) sent as x-ozone-agent.
 *  - READ tools never change anything. MUTATING tools (ozone_orchestrate, ozone_link) are marked in their
 *    description and each one appends a shared-context note (same .ozone-context/notes.jsonl the
 *    ozone-shared-context server uses) holding only the agent, the tool, a content hash and lengths —
 *    never the prompt text or file contents.
 *  - Marketplace decide/accept is operator-only and NOT exposed. Task cancel is not exposed either.
 *  - Long walks never block: ozone_orchestrate returns a run_token at once; poll ozone_run_status.
 *  - No size refusals: any tool result can be read in chunks with offset/length (omit = whole result).
 *  - Failures are returned as real errors (isError) with the host's own message. Nothing is fabricated.
 *
 * Participants (docs/evidence/mcp-participants/REPORT.md): an MCP call can BE the prompt. ozone_orchestrate names the
 * caller (this server's bound agent) and optional `participants`; the host records them with the walk. Every
 * participant reads the walk through ozone_walk_feed (stage events, status, work-order changes, from a cursor — also
 * mirrored into the shared context dir under walks/, git-ignored) and discovers its walks with ozone_walk_mine. A step
 * whose provider is an AGENT (ozone_agent_offer registers one) is handed to that agent: it sees the request in
 * ozone_agent_requests and its ozone_agent_report completes the step. Gates await; nothing fails open.
 * Identity (H1): one server process = one agent. The `agent` argument can only ever equal OZONE_AGENT_NAME (set it per
 * agent); another name is refused. Per-agent device key: OZONE_AGENT_KEY_PATH, else ~/.ozone-cli/agents/<name>/device-key.pem
 * when it exists, else the shared key (reported as shared by ozone_whoami). OZONE_MCP_READ_ONLY=1 makes the agent
 * read-only (mutating tools refused). `node server.mjs --register` registers this server with the host (H10).
 *
 * Protocol: MCP 2024-11-05, newline-delimited JSON-RPC 2.0 on stdio.
 * Env: OZONE_HOST (default http://127.0.0.1:50051; a non-loopback host needs OZONE_MCP_ALLOW_REMOTE_HOST=1),
 *      OZONE_AGENT_NAME, OZONE_USER_ID/OZONE_DEVICE_ID (default 1), OZONE_AGENT_KEY_PATH (default
 *      ~/.ozone-cli/device-key.pem), OZONE_CONTEXT_DIR (default ./.ozone-context),
 *      OZONE_MCP_LINK_ROOTS (":"-separated roots the link tool may read; default = the working directory).
 */
import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const HOST = (process.env.OZONE_HOST || "http://127.0.0.1:50051").replace(/\/+$/, "");
const BOUND_AGENT = process.env.OZONE_AGENT_NAME || null; // set per agent: the identity this process may speak as (H1)
const AGENT_NAME = BOUND_AGENT || "mcp-agent";
const READ_ONLY = ["1", "true", "on", "yes"].includes(String(process.env.OZONE_MCP_READ_ONLY || "").toLowerCase());
const ALLOW_AGENT_OVERRIDE = process.env.OZONE_MCP_ALLOW_AGENT_OVERRIDE === "1"; // explicit opt-out of H1 binding
const USER_ID = Number(process.env.OZONE_USER_ID || 1);
const DEVICE_ID = Number(process.env.OZONE_DEVICE_ID || 1);
// per-agent key (H1): explicit path, else ~/.ozone-cli/agents/<name>/device-key.pem when present, else the shared key
const PER_AGENT_KEY = path.join(os.homedir(), ".ozone-cli", "agents", AGENT_NAME, "device-key.pem");
const KEY_PATH = process.env.OZONE_AGENT_KEY_PATH || (BOUND_AGENT && fs.existsSync(PER_AGENT_KEY) ? PER_AGENT_KEY : path.join(os.homedir(), ".ozone-cli", "device-key.pem"));
const KEY_MODE = process.env.OZONE_AGENT_KEY_PATH || (BOUND_AGENT && fs.existsSync(PER_AGENT_KEY)) ? "per-agent" : "shared";
const STATE_DIR = process.env.OZONE_CONTEXT_DIR || path.join(process.cwd(), ".ozone-context");
const LINK_ROOTS = (process.env.OZONE_MCP_LINK_ROOTS || process.cwd()).split(":").filter(Boolean);
const FILE_LINK_PIPELINE_ID = 30; // src/pipeline/registry.rs: 30 = FileLink

// ── host check (SSRF): the host is fixed by env, never by a tool argument ─────────────────────────────
function assertHostAllowed() {
  const u = new URL(HOST);
  const loop = ["127.0.0.1", "localhost", "::1", "[::1]"].includes(u.hostname);
  if (!loop && process.env.OZONE_MCP_ALLOW_REMOTE_HOST !== "1") {
    throw new Error(`OZONE_HOST ${u.hostname} is not loopback; set OZONE_MCP_ALLOW_REMOTE_HOST=1 to allow a remote host deliberately`);
  }
}

// ── session (device-key challenge, same as tools/e2e/lib.mjs) ─────────────────────────────────────────
let session = null;
async function rawHttp(method, p, body, headers = {}) {
  const r = await fetch(HOST + p, {
    method,
    headers: { "content-type": "application/json", ...headers },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await r.text();
  let json;
  try { json = JSON.parse(text); } catch { json = text; }
  return { status: r.status, json };
}
async function sessionToken(force = false) {
  if (session && !force) return session;
  assertHostAllowed();
  let pem;
  try { pem = fs.readFileSync(KEY_PATH, "utf8"); } catch {
    throw new Error(`no device key at ${KEY_PATH}; pair this machine first (tools/ozone-cli) or set OZONE_AGENT_KEY_PATH`);
  }
  const priv = crypto.createPrivateKey(pem);
  const spki = crypto.createPublicKey(priv).export({ type: "spki", format: "der" });
  const pub = spki.subarray(spki.length - 32).toString("hex");
  const ch = (await rawHttp("POST", "/auth/challenge", { public_key: pub })).json;
  if (!ch?.challenge) throw new Error("host gave no auth challenge (is this device paired?)");
  const sig = crypto.sign(null, Buffer.from(ch.challenge, "hex"), priv).toString("hex");
  const au = (await rawHttp("POST", "/auth/authenticate", { public_key: pub, signature: sig })).json;
  if (!au?.session_token) throw new Error("authentication refused by the host");
  return (session = au.session_token);
}
function scrub(s) {
  let out = String(s);
  if (session) out = out.split(session).join("[session]");
  return out;
}
async function api(agent, method, p, body) {
  const call = async (force) => {
    const tok = await sessionToken(force);
    return rawHttp(method, p, body, { authorization: `Bearer ${tok}`, "x-ozone-agent": agent });
  };
  let r = await call(false);
  if (r.status === 401) r = await call(true); // session expired: re-authenticate once
  return r;
}

// ── shared context notes (append-only, same file/format as ozone-shared-context) ──────────────────────
function recordMutation(agent, tool, detail) {
  try {
    fs.mkdirSync(STATE_DIR, { recursive: true });
    const note = {
      id: Date.now().toString(36) + Math.random().toString(36).slice(2, 6),
      agent,
      kind: "finding",
      title: `ozone-mcp: ${tool}`,
      body: JSON.stringify(detail),
      files: [],
      scope: "workspace",
      at: Date.now(),
    };
    fs.appendFileSync(path.join(STATE_DIR, "notes.jsonl"), JSON.stringify(note) + "\n");
    return note.id;
  } catch (e) {
    return null; // coordination note failure never hides the real result; the result reports note_id:null
  }
}
const sha = (s) => crypto.createHash("sha256").update(String(s)).digest("hex").slice(0, 16);

// ── argument validation ──────────────────────────────────────────────────────────────────────────────
const AGENT_RE = /^[A-Za-z0-9._-]{1,64}$/;
function agentOf(a) {
  const v = a.agent ?? AGENT_NAME;
  if (!AGENT_RE.test(v)) throw new Error("agent must match [A-Za-z0-9._-]{1,64}");
  // H1: one process = one agent. An argument can never make this process speak as someone else.
  if (BOUND_AGENT && v !== BOUND_AGENT && !ALLOW_AGENT_OVERRIDE) {
    throw new Error(`this server is bound to agent '${BOUND_AGENT}'; it will not act as '${v}' (OZONE_MCP_ALLOW_AGENT_OVERRIDE=1 to override deliberately)`);
  }
  return v;
}
const MUTATING_TOOLS = new Set(["ozone_orchestrate", "ozone_link", "ozone_agent_report", "ozone_agent_offer"]);
function uint(v, name, required = false) {
  if (v === undefined || v === null) { if (required) throw new Error(`${name} is required`); return undefined; }
  const n = Number(v);
  if (!Number.isSafeInteger(n) || n < 0) throw new Error(`${name} must be a non-negative integer`);
  return n;
}
function qs(obj) {
  const p = Object.entries(obj).filter(([, v]) => v !== undefined && v !== null && v !== "");
  return p.length ? "?" + p.map(([k, v]) => `${encodeURIComponent(k)}=${encodeURIComponent(String(v))}`).join("&") : "";
}

// ── link-path hardening: absolute, real path, inside an allowed root, symlinks resolved ──────────────
function checkLinkPath(p, kind) {
  if (typeof p !== "string" || !p) throw new Error("path is required");
  if (p.includes("\0")) throw new Error("path contains a NUL byte");
  if (!path.isAbsolute(p)) throw new Error("path must be absolute");
  let real;
  try { real = fs.realpathSync(p); } catch { throw new Error(`path does not exist: ${p}`); }
  const roots = LINK_ROOTS.map((r) => { try { return fs.realpathSync(r); } catch { return null; } }).filter(Boolean);
  if (!roots.some((r) => real === r || real.startsWith(r + path.sep))) {
    throw new Error(`path is outside the allowed link roots (OZONE_MCP_LINK_ROOTS); resolved to ${real}`);
  }
  const st = fs.statSync(real);
  if (kind === "file" && !st.isFile()) throw new Error("path is not a regular file");
  if (kind === "folder" && !st.isDirectory()) throw new Error("path is not a directory");
  return real;
}

// ── chunked results: no size refusal; offset/length read any result in pieces ────────────────────────
function chunked(value, a) {
  const text = typeof value === "string" ? value : JSON.stringify(value, null, 2);
  const off = uint(a.offset, "offset");
  const len = uint(a.length, "length");
  if (off === undefined && len === undefined) return text;
  const start = off ?? 0;
  const end = len === undefined ? text.length : Math.min(text.length, start + len);
  return JSON.stringify({ chunk: text.slice(start, end), offset: start, next_offset: end < text.length ? end : null, total_chars: text.length });
}
const CHUNK_PROPS = {
  offset: { type: "number", description: "optional: start character of the result to return (chunked read)" },
  length: { type: "number", description: "optional: number of characters to return from offset" },
  agent: { type: "string", description: "caller identity; defaults to this server's OZONE_AGENT_NAME" },
};
const obj = (properties = {}, required = []) => ({ type: "object", properties: { ...properties, ...CHUNK_PROPS }, required });

// read-only ZSEI variants only; anything else is refused loudly (writes go through the walk / link tool)
const READ_QUERIES = new Set([
  "GetUserWorkspaces", "GetProjects", "GetProjectContext", "GetContainer", "GetContainerContent",
  "GetFileReferences", "GetExternalReferences", "GetCategories", "GetMethodologiesByKeywords",
  "SearchContainersByKeywords", "GetWorkspaceContext",
]);

// ── shared walk context (participants read the same files; rows are git-ignored, notes carry hashes only) ──────
const WALK_DIR = path.join(STATE_DIR, "walks");
const walkFile = (tok) => path.join(WALK_DIR, `${tok}.jsonl`);
const TOKEN_RE = /^[A-Za-z0-9._-]{1,128}$/;
function readJsonl(file) {
  try { return fs.readFileSync(file, "utf8").split("\n").filter(Boolean).map((l) => { try { return JSON.parse(l); } catch { return null; } }).filter(Boolean); } catch { return []; }
}
function appendWalkRows(tok, rows) {
  if (!rows.length) return;
  fs.mkdirSync(WALK_DIR, { recursive: true });
  fs.appendFileSync(walkFile(tok), rows.map((r) => JSON.stringify(r)).join("\n") + "\n");
}
function indexWalk(entry) {
  try { fs.mkdirSync(WALK_DIR, { recursive: true }); fs.appendFileSync(path.join(WALK_DIR, "index.jsonl"), JSON.stringify(entry) + "\n"); } catch { /* the host record is the truth */ }
}
/** Flatten a work-order tree into [{key,title,status}] — shape read defensively, nothing invented. */
function flattenItems(node, out = []) {
  if (!node || typeof node !== "object") return out;
  if (Array.isArray(node)) { node.forEach((n) => flattenItems(n, out)); return out; }
  if (node.status && (node.item_id ?? node.id ?? node.title ?? node.name)) out.push({ key: String(node.item_id ?? node.id ?? node.title ?? node.name), title: node.title ?? node.name ?? null, status: String(node.status) });
  for (const v of Object.values(node)) if (v && typeof v === "object") flattenItems(v, out);
  return out;
}
const slim = (st) => st && typeof st === "object" ? Object.fromEntries(Object.entries(st).filter(([k]) => k !== "result")) : st;

/**
 * Pull everything new about one walk from the host and append it to the SHARED walk file: stage rows (cursor = last
 * host seq seen), status transitions, work-order item changes. Any participant's call advances it; the starter's
 * background loop does too. Returns what was appended.
 */
async function advanceWalk(agent, tok) {
  const file = walkFile(tok);
  const rows = readJsonl(file);
  const lastSeq = rows.reduce((m, r) => (r.src === "host" && Number.isInteger(r.seq) ? Math.max(m, r.seq + 1) : m), 0);
  const added = [];
  const feed = unwrapSoft(await api(agent, "GET", `/walk/feed${qs({ run_token: tok, after: lastSeq })}`));
  if (feed?.found) for (const r of feed.rows ?? []) added.push({ ...r, src: "host" });
  const status = unwrapSoft(await api(agent, "GET", `/orchestrate/status${qs({ run_token: tok })}`));
  const lastStatus = [...rows].reverse().find((r) => r.kind === "status")?.state;
  if (status?.found && status.state !== lastStatus) added.push({ kind: "status", state: status.state, task_id: status.task_id ?? null, awaiting: !!status.awaiting, await_id: status.await_id ?? null, success: status.success ?? null, error: status.error ?? null, ts: Math.floor(Date.now() / 1000) });
  const taskId = status?.task_id ?? feed?.walk?.task_id;
  if (taskId) {
    const wo = unwrapSoft(await api(agent, "GET", `/order/work/${Number(taskId)}`));
    const now = flattenItems(wo?.work_order ?? wo);
    const prev = new Map();
    for (const r of rows) if (r.kind === "work_item") prev.set(r.key, r.status);
    for (const it of now) if (prev.get(it.key) !== it.status) added.push({ kind: "work_item", key: it.key, title: it.title, status: it.status, task_id: Number(taskId), ts: Math.floor(Date.now() / 1000) });
  }
  appendWalkRows(tok, added);
  const finalSeen = status?.found && status.state && status.state !== "running";
  if (finalSeen && !fs.existsSync(path.join(WALK_DIR, `${tok}.done`))) {
    try { fs.writeFileSync(path.join(WALK_DIR, `${tok}.done`), String(Date.now())); } catch { /* marker only */ }
    recordMutation(agent, "ozone_walk_state", { run_token: tok, state: status.state, task_id: status.task_id ?? null, awaiting: !!status.awaiting });
  }
  return { added: added.length, state: status?.state ?? null, final: !!finalSeen, task_id: taskId ?? null };
}
/** A tolerant read for the feed path: a missing route or transient error becomes null, recorded by the caller as no rows. */
function unwrapSoft(r) { return r.status >= 400 ? null : r.json; }

// background participant feed: keeps the shared walk file current for every participant while this server lives.
// Adaptive pacing (no fixed cap on how long a walk may run): quick while rows arrive, slower while idle.
const watching = new Set();
function watchWalk(agent, tok) {
  if (watching.has(tok)) return;
  watching.add(tok);
  (async () => {
    let delay = 1500, quiet = 0;
    for (;;) {
      await new Promise((r) => setTimeout(r, delay));
      let res;
      try { res = await advanceWalk(agent, tok); } catch (e) { appendWalkRows(tok, [{ kind: "watch_error", error: scrub(e.message), ts: Math.floor(Date.now() / 1000) }]); res = { added: 0, final: false }; }
      if (res.final && !res.awaiting) { /* keep polling at the slow pace: awaiting/parked walks can resume */ }
      quiet = res.added ? 0 : quiet + 1;
      delay = res.added ? 1500 : Math.min(30000, 1500 * (1 + quiet));
      if (res.final && quiet > 40) break; // finished and silent for a long while: stop (any participant can still advance on demand)
    }
    watching.delete(tok);
  })();
}

// H6: a path another LIVE agent has claimed is not linked out from under them
function claimConflict(real, agent) {
  try {
    const st = JSON.parse(fs.readFileSync(path.join(STATE_DIR, "state.json"), "utf8"));
    for (const [file, c] of Object.entries(st.claims ?? {})) {
      if (c.agent === agent) continue;
      const live = Date.now() - ((st.sessions ?? {})[c.agent]?.last_seen ?? 0) < 5 * 60 * 1000;
      if (!live) continue;
      const claimed = path.resolve(process.cwd(), file);
      if (real === claimed || real.startsWith(claimed + path.sep)) return { file, held_by: c.agent, reason: c.reason };
    }
  } catch { /* no coordination state: no claims */ }
  return null;
}

// ── tools ─────────────────────────────────────────────────────────────────────────────────────────────
const TOOLS = [
  { name: "ozone_health", description: "READ. Host health (GET /health).", inputSchema: obj() },
  { name: "ozone_task_list", description: "READ. Tasks, incl. kind / parent_task_id / rerun_of (POST /task/list).", inputSchema: obj({ status: { type: "string" }, limit: { type: "number" }, offset_rows: { type: "number", description: "row offset into the task list (offset/length chunk the returned text)" } }) },
  { name: "ozone_task_get", description: "READ. One task (POST /task/get).", inputSchema: obj({ task_id: { type: "number" } }, ["task_id"]) },
  { name: "ozone_work_order", description: "READ. The work-order tree of a task: checklist = step = task completion (GET /order/work/<task_id>).", inputSchema: obj({ task_id: { type: "number" } }, ["task_id"]) },
  { name: "ozone_run_status", description: "READ. Status of a walk started with ozone_orchestrate (GET /orchestrate/status). state is reported as the host gives it (running, completed, awaiting, failed ...).", inputSchema: obj({ run_token: { type: "string" } }, ["run_token"]) },
  { name: "ozone_callgraph_system", description: "READ. The host's own call graphs: walk stages, loops, call routes (GET /callgraph/system).", inputSchema: obj() },
  { name: "ozone_callgraph_live", description: "READ. What actually ran in one walk, joined by trace id (GET /callgraph/live). Give task_id, trace_id or latest=true.", inputSchema: obj({ task_id: { type: "number" }, trace_id: { type: "string" }, latest: { type: "boolean" } }) },
  { name: "ozone_callgraph_files", description: "READ. Graphs the host created by itself for linked files (graph_on_link ledger: status, content hash, graph id, per-modality entry meaning) (GET /callgraph/files).", inputSchema: obj({ project_id: { type: "number" } }) },
  { name: "ozone_gaps", description: "READ. Capability gaps and their provenance (GET /capability/gaps).", inputSchema: obj() },
  { name: "ozone_refinement", description: "READ. Methodology refinement report (GET /methodology/refinement).", inputSchema: obj() },
  { name: "ozone_marketplace_view", description: "READ. Marketplace overview: peers, offers with local classification, policy (GET /marketplace/overview). Accepting/deciding offers is operator-only and not available here.", inputSchema: obj() },
  { name: "ozone_query", description: `READ. A read-only ZSEI query (POST /zsei/query). Pass the single-variant query object, e.g. {"GetProjects":{"workspace_id":1}}. Allowed variants: ${[...READ_QUERIES].join(", ")}.`, inputSchema: obj({ query: { type: "object" } }, ["query"]) },
  {
    name: "ozone_orchestrate",
    description: "MUTATING. Start a real walk (POST /orchestrate, submit_only). Returns immediately with run_token; poll ozone_run_status, then ozone_task_get / ozone_work_order / ozone_callgraph_live. Spends model calls on the host's configured chain. Records a note (agent, prompt hash and length only).",
    inputSchema: obj({
      prompt: { type: "string" }, workspace_id: { type: "number" }, project_id: { type: "number" },
      token_budget: { type: "number" }, consciousness_enabled: { type: "boolean" },
      attached_files: { type: "array", items: { type: "string" }, description: "absolute paths inside the allowed link roots; the host reads them itself" },
      participants: { type: "array", items: { type: "string" }, description: "other agents taking part in this walk (the caller is always the first). They read the walk with ozone_walk_feed and can answer agent-provider steps addressed to them or to agent:*." },
    }, ["prompt"]),
  },
  { name: "ozone_whoami", description: "READ. This server's agent identity: bound name, key mode (per-agent | shared), host, read-only switch. Never returns a key or token.", inputSchema: obj() },
  { name: "ozone_walk_feed", description: "READ. The live state of a walk you take part in, from a cursor: stage events, status transitions, work-order item changes, plus the participants. Give run_token (or task_id). Returns {walk, rows, next, state}; pass next back as `after`. Also mirrored for every participant into the shared context dir (walks/, git-ignored). This is how a participant stays aware of the walk it is making.", inputSchema: obj({ run_token: { type: "string" }, task_id: { type: "number" }, after: { type: "number", description: "cursor from the previous call (row index in the shared walk file); omit for everything" } }) },
  { name: "ozone_walk_mine", description: "READ. The walks this agent participates in (as caller or named participant), newest first, from the host's record.", inputSchema: obj() },
  { name: "ozone_agent_requests", description: "READ. Walk steps handed to THIS agent that await its report (agent-provider steps addressed to it, or to agent:* in a walk it participates in). Each has an id, the step's request text, and the walk's task/trace.", inputSchema: obj() },
  { name: "ozone_agent_report", description: "MUTATING. Report the result of an agent-provider request (id from ozone_agent_requests). The report COMPLETES the awaiting step; completed is final (a second report is refused). result is any JSON; include success:false and error to report a failure honestly. Records a note (hashes only).", inputSchema: obj({ id: { type: "string" }, result: { type: ["object", "string", "array", "number", "boolean"] } }, ["id", "result"]) },
  { name: "ozone_agent_offer", description: "MUTATING. Register this agent as a PROVIDER the walks can use: a provider step naming `agent:<this agent>` is handed to it and awaits its report. capabilities describe what it can do (they are matched against step text like any provider's tools). Records a note.", inputSchema: obj({ capabilities: { type: "array", items: { type: "string" } }, name: { type: "string", description: "tool row name; default agent_<agent>" } }, ["capabilities"]) },
  {
    name: "ozone_link",
    description: "MUTATING. Link a file or a folder (one level) to a project through the FileLink pipeline (#30). The host then builds the non-semantic graph on its own (graph_on_link); read it with ozone_callgraph_files. Paths must be absolute and inside OZONE_MCP_LINK_ROOTS. Records a note (agent, kind, path hash).",
    inputSchema: obj({
      kind: { type: "string", enum: ["file", "folder"] }, path: { type: "string" },
      project_id: { type: "number" }, workspace_id: { type: "number" }, zsei_parent_id: { type: "number", description: "real ZSEI container id of the project, when known" },
      analyze: { type: "boolean", description: "default true" },
    }, ["kind", "path", "project_id"]),
  },
];

async function callTool(name, a) {
  const agent = agentOf(a);
  if (READ_ONLY && MUTATING_TOOLS.has(name)) throw new Error(`this agent is read-only (OZONE_MCP_READ_ONLY=1): ${name} is a mutating tool`);
  const out = (v) => chunked(v, a);
  const get = async (p) => out(unwrap(await api(agent, "GET", p), p));
  const post = async (p, b) => out(unwrap(await api(agent, "POST", p, b), p));
  switch (name) {
    case "ozone_health": return get("/health");
    case "ozone_task_list": return post("/task/list", { status: a.status ?? null, limit: uint(a.limit, "limit") ?? null, offset: uint(a.offset_rows, "offset_rows") ?? 0, session_token: await sessionToken() });
    case "ozone_task_get": return post("/task/get", { task_id: uint(a.task_id, "task_id", true), session_token: await sessionToken() });
    case "ozone_work_order": return get(`/order/work/${uint(a.task_id, "task_id", true)}`);
    case "ozone_run_status": {
      if (typeof a.run_token !== "string" || !/^[A-Za-z0-9._-]{1,128}$/.test(a.run_token)) throw new Error("run_token has an invalid form");
      return get(`/orchestrate/status${qs({ run_token: a.run_token })}`);
    }
    case "ozone_callgraph_system": return get("/callgraph/system");
    case "ozone_callgraph_live": {
      const q = { task_id: uint(a.task_id, "task_id"), trace_id: a.trace_id, latest: a.latest ? 1 : undefined };
      if (q.task_id === undefined && !q.trace_id && !q.latest) throw new Error("give task_id, trace_id or latest=true");
      if (q.trace_id && !/^[A-Za-z0-9._:-]{1,128}$/.test(q.trace_id)) throw new Error("trace_id has an invalid form");
      return get("/callgraph/live" + qs(q));
    }
    case "ozone_callgraph_files": return get("/callgraph/files" + qs({ project_id: uint(a.project_id, "project_id") }));
    case "ozone_gaps": return get("/capability/gaps");
    case "ozone_refinement": return get("/methodology/refinement");
    case "ozone_marketplace_view": return get("/marketplace/overview");
    case "ozone_query": {
      const variants = a.query && typeof a.query === "object" ? Object.keys(a.query) : [];
      if (variants.length !== 1) throw new Error("query must be an object with exactly one variant");
      if (!READ_QUERIES.has(variants[0])) throw new Error(`ZSEI variant '${variants[0]}' is not a read-only query exposed here (allowed: ${[...READ_QUERIES].join(", ")})`);
      return post("/zsei/query", { query: a.query, session_token: await sessionToken() });
    }
    case "ozone_orchestrate": {
      if (typeof a.prompt !== "string" || !a.prompt.trim()) throw new Error("prompt is required");
      const attached = (a.attached_files ?? []).map((p) => checkLinkPath(p, "file"));
      const participants = (a.participants ?? []).map((p) => { if (!AGENT_RE.test(p)) throw new Error(`participant '${p}' must match [A-Za-z0-9._-]{1,64}`); return p; });
      const body = {
        prompt: a.prompt, user_id: USER_ID, device_id: DEVICE_ID, session_token: await sessionToken(),
        agent, participants,
        workspace_id: uint(a.workspace_id, "workspace_id") ?? null, project_id: uint(a.project_id, "project_id") ?? null,
        token_budget: uint(a.token_budget, "token_budget") ?? null,
        attached_files: attached.map((p) => ({ file_path: p, mime_type: null, is_inline: false, content_preview: null })), submit_only: true,
      };
      if (typeof a.consciousness_enabled === "boolean") body.consciousness_enabled = a.consciousness_enabled;
      const r = unwrap(await api(agent, "POST", "/orchestrate", body), "/orchestrate");
      const note_id = recordMutation(agent, "ozone_orchestrate", { prompt_sha: sha(a.prompt), prompt_chars: a.prompt.length, attached: attached.length, participants, workspace_id: body.workspace_id, project_id: body.project_id, run_token: r?.run_token ?? null });
      if (r?.run_token && TOKEN_RE.test(r.run_token)) {
        // the walk is announced to every participant through the shared context dir (hashes + names only) and the feed starts
        indexWalk({ run_token: r.run_token, caller: agent, participants: [agent, ...participants.filter((p) => p !== agent)], prompt_sha: sha(a.prompt), prompt_chars: a.prompt.length, at: Date.now() });
        appendWalkRows(r.run_token, [{ kind: "walk_started", caller: agent, participants: [agent, ...participants.filter((p) => p !== agent)], ts: Math.floor(Date.now() / 1000) }]);
        watchWalk(agent, r.run_token);
      }
      return out({ ...(typeof r === "object" ? r : { response: r }), note_id, participants: [agent, ...participants.filter((p) => p !== agent)], feed: "ozone_walk_feed(run_token)" });
    }
    case "ozone_link": {
      const kind = a.kind;
      if (kind !== "file" && kind !== "folder") throw new Error("kind must be 'file' or 'folder'");
      const real = checkLinkPath(a.path, kind);
      const conflict = claimConflict(real, agent);
      if (conflict) throw new Error(`path is claimed by live agent '${conflict.held_by}' (${conflict.file}: ${conflict.reason ?? "no reason"}); coordinate through shared context before linking`);
      const project_id = uint(a.project_id, "project_id", true);
      const data = kind === "file"
        ? { action: "Link", project_id, file_path: real, analyze: a.analyze !== false, zsei_parent_id: uint(a.zsei_parent_id, "zsei_parent_id") ?? null }
        : { action: "LinkFolder", project_id, folder_path: real, analyze: a.analyze !== false, zsei_parent_id: uint(a.zsei_parent_id, "zsei_parent_id") ?? null };
      const body = {
        pipeline_id: FILE_LINK_PIPELINE_ID,
        input: { data, context: { user_id: USER_ID, device_id: DEVICE_ID, workspace_id: uint(a.workspace_id, "workspace_id") ?? null, project_id, task_context_id: null, metadata: {} } },
        session_token: await sessionToken(),
      };
      const r = unwrap(await api(agent, "POST", "/pipeline/execute", body), "/pipeline/execute");
      const note_id = recordMutation(agent, "ozone_link", { kind, path_sha: sha(real), project_id });
      return out({ result: r, note_id });
    }
    case "ozone_whoami": return out({ agent, bound: !!BOUND_AGENT, key_mode: KEY_MODE, host: HOST, read_only: READ_ONLY, link_roots: LINK_ROOTS });
    case "ozone_walk_mine": return get(`/walk/mine${qs({ agent })}`);
    case "ozone_walk_feed": {
      let tok = a.run_token;
      if (tok === undefined && a.task_id !== undefined) {
        const w = unwrap(await api(agent, "GET", `/walk/feed${qs({ task_id: uint(a.task_id, "task_id") })}`), "/walk/feed");
        tok = w?.walk?.run_token;
        if (!tok) throw new Error("no walk with that task_id has a run_token on record (the blocking /orchestrate path has none)");
      }
      if (typeof tok !== "string" || !TOKEN_RE.test(tok)) throw new Error("run_token (or task_id) is required");
      const adv = await advanceWalk(agent, tok);
      const rows = readJsonl(walkFile(tok));
      const after = uint(a.after, "after") ?? 0;
      const w = unwrapSoft(await api(agent, "GET", `/walk/feed${qs({ run_token: tok, after: 1e15 })}`));
      return out({ run_token: tok, walk: w?.walk ?? null, state: adv.state, final: adv.final, task_id: adv.task_id, rows: rows.slice(after), next: rows.length });
    }
    case "ozone_agent_requests": return get(`/agent/requests${qs({ agent })}`);
    case "ozone_agent_report": {
      if (typeof a.id !== "string" || !/^ar-[0-9a-f]{16}$/.test(a.id)) throw new Error("id must be an agent request id (ar-…)");
      const r = unwrap(await api(agent, "POST", "/agent/report", { id: a.id, agent, result: a.result, session_token: await sessionToken() }), "/agent/report");
      const note_id = recordMutation(agent, "ozone_agent_report", { id: a.id, result_sha: sha(JSON.stringify(a.result ?? null)), result_chars: JSON.stringify(a.result ?? null).length });
      return out({ ...r, note_id });
    }
    case "ozone_agent_offer": {
      const caps = (a.capabilities ?? []).map(String).filter(Boolean);
      if (!caps.length) throw new Error("capabilities is required (what this agent can do, as plain phrases)");
      const rowName = a.name ?? `agent_${agent}`;
      if (!/^[A-Za-z0-9._-]{1,96}$/.test(rowName)) throw new Error("name must match [A-Za-z0-9._-]{1,96}");
      const r = unwrap(await api(agent, "POST", "/mcp/tools/register", { name: rowName, transport: "agent", endpoint: `agent:${agent}`, capabilities: caps, server_version: "ozone-mcp-0.2" }), "/mcp/tools/register");
      const note_id = recordMutation(agent, "ozone_agent_offer", { name: rowName, capabilities: caps.length });
      return out({ ...r, provider: `agent:${agent}`, note_id });
    }
    default: throw new Error(`unknown tool: ${name}`);
  }
}

/** A non-2xx or a host-level error is thrown with the host's own message — never turned into an empty success. */
function unwrap(r, p) {
  if (r.status >= 400) throw new Error(`${p} -> HTTP ${r.status}: ${typeof r.json === "string" ? r.json : JSON.stringify(r.json)}`);
  if (r.json && typeof r.json === "object" && r.json.success === false) throw new Error(`${p} -> ${r.json.error ?? "success:false"}`);
  return r.json;
}

// H10: `node server.mjs --register` — an operator-run command that registers this server's tools with the host through a
// real session (not through the data-dir token). One row, stdio transport (the host never spawns it), capabilities = tool names.
if (process.argv.includes("--register")) {
  (async () => {
    const r = await api(AGENT_NAME, "POST", "/mcp/tools/register", {
      name: "ozone-mcp", transport: "stdio", endpoint: `node ${path.resolve(process.argv[1])}`,
      capabilities: TOOLS.map((t) => t.name), server_version: "ozone-mcp-0.2",
    });
    process.stdout.write(`${JSON.stringify(unwrap(r, "/mcp/tools/register"))}\n`);
    process.exit(0);
  })().catch((e) => { process.stderr.write(`register failed: ${scrub(e.message)}\n`); process.exit(1); });
}

// ── JSON-RPC over stdio ───────────────────────────────────────────────────────────────────────────────
const reply = (id, result) => process.stdout.write(JSON.stringify({ jsonrpc: "2.0", id, result }) + "\n");
const replyError = (id, code, message) => process.stdout.write(JSON.stringify({ jsonrpc: "2.0", id, error: { code, message } }) + "\n");

async function handle(msg) {
  const { id, method, params } = msg;
  if (method === "initialize") return reply(id, { protocolVersion: params?.protocolVersion || "2024-11-05", capabilities: { tools: {} }, serverInfo: { name: "ozone-mcp", version: "0.1.0" } });
  if (typeof method === "string" && method.startsWith("notifications/")) return;
  if (method === "ping") return reply(id, {});
  if (method === "tools/list") return reply(id, { tools: TOOLS });
  if (method === "tools/call") {
    try {
      const text = await callTool(params.name, params.arguments || {});
      return reply(id, { content: [{ type: "text", text }] });
    } catch (e) {
      return reply(id, { content: [{ type: "text", text: `error: ${scrub(e.message)}` }], isError: true });
    }
  }
  if (id !== undefined) replyError(id, -32601, `method not found: ${method}`);
}

let buffer = "";
process.stdin.setEncoding("utf8");
process.stdin.on("data", (chunk) => {
  buffer += chunk;
  let i;
  while ((i = buffer.indexOf("\n")) >= 0) {
    const line = buffer.slice(0, i).trim();
    buffer = buffer.slice(i + 1);
    if (!line) continue;
    let msg;
    try { msg = JSON.parse(line); } catch { continue; }
    handle(msg).catch((e) => { if (msg.id !== undefined) replyError(msg.id, -32603, `internal error: ${scrub(e.message)}`); });
  }
});
process.stderr.write(`[ozone-mcp] ready — agent ${AGENT_NAME}, host ${HOST}, notes in ${STATE_DIR}\n`);
