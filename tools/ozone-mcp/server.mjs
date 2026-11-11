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
const AGENT_NAME = process.env.OZONE_AGENT_NAME || "mcp-agent";
const USER_ID = Number(process.env.OZONE_USER_ID || 1);
const DEVICE_ID = Number(process.env.OZONE_DEVICE_ID || 1);
const KEY_PATH = process.env.OZONE_AGENT_KEY_PATH || path.join(os.homedir(), ".ozone-cli", "device-key.pem");
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
  return v;
}
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
    }, ["prompt"]),
  },
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
      const body = {
        prompt: a.prompt, user_id: USER_ID, device_id: DEVICE_ID, session_token: await sessionToken(),
        workspace_id: uint(a.workspace_id, "workspace_id") ?? null, project_id: uint(a.project_id, "project_id") ?? null,
        token_budget: uint(a.token_budget, "token_budget") ?? null,
        attached_files: attached.map((p) => ({ file_path: p, mime_type: null, is_inline: false, content_preview: null })), submit_only: true,
      };
      if (typeof a.consciousness_enabled === "boolean") body.consciousness_enabled = a.consciousness_enabled;
      const r = unwrap(await api(agent, "POST", "/orchestrate", body), "/orchestrate");
      const note_id = recordMutation(agent, "ozone_orchestrate", { prompt_sha: sha(a.prompt), prompt_chars: a.prompt.length, attached: attached.length, workspace_id: body.workspace_id, project_id: body.project_id, run_token: r?.run_token ?? null });
      return out({ ...(typeof r === "object" ? r : { response: r }), note_id });
    }
    case "ozone_link": {
      const kind = a.kind;
      if (kind !== "file" && kind !== "folder") throw new Error("kind must be 'file' or 'folder'");
      const real = checkLinkPath(a.path, kind);
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
    default: throw new Error(`unknown tool: ${name}`);
  }
}

/** A non-2xx or a host-level error is thrown with the host's own message — never turned into an empty success. */
function unwrap(r, p) {
  if (r.status >= 400) throw new Error(`${p} -> HTTP ${r.status}: ${typeof r.json === "string" ? r.json : JSON.stringify(r.json)}`);
  if (r.json && typeof r.json === "object" && r.json.success === false) throw new Error(`${p} -> ${r.json.error ?? "success:false"}`);
  return r.json;
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
