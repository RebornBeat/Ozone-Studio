// MCP test harness (outside client). Spawns real MCP servers over stdio and speaks newline-delimited JSON-RPC 2.0,
// exactly as Claude Code / ZCode do. No mocks: the servers are the real ozone-shared-context and ozone-mcp
// processes, talking to a real Ozone-Studio host. Not part of production code; nothing here is imported by src/.
import { spawn } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { suite as baseSuite } from "../tpv.mjs";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, "../../..");
export const SERVERS = {
  shared: path.join(ROOT, "tools/ozone-shared-context/server.js"),
  ozone: path.join(ROOT, "tools/ozone-mcp/server.mjs"),
};

/** One stdio MCP client = one server process = one agent identity. */
export class McpClient {
  constructor(serverPath, env = {}) {
    this.proc = spawn(process.execPath, [serverPath], { env: { ...process.env, ...env }, stdio: ["pipe", "pipe", "pipe"] });
    this.nextId = 1; this.pending = new Map(); this.buf = ""; this.stderr = "";
    this.proc.stdout.setEncoding("utf8");
    this.proc.stdout.on("data", (c) => {
      this.buf += c; let i;
      while ((i = this.buf.indexOf("\n")) >= 0) {
        const line = this.buf.slice(0, i).trim(); this.buf = this.buf.slice(i + 1);
        if (!line) continue;
        let m; try { m = JSON.parse(line); } catch { continue; }
        const p = this.pending.get(m.id); if (p) { this.pending.delete(m.id); p(m); }
      }
    });
    this.proc.stderr.on("data", (c) => { this.stderr += c; });
  }
  rpc(method, params, timeoutMs = 120000) {
    const id = this.nextId++;
    return new Promise((resolve, reject) => {
      const t = setTimeout(() => { this.pending.delete(id); reject(new Error(`${method} timed out after ${timeoutMs}ms`)); }, timeoutMs);
      this.pending.set(id, (m) => { clearTimeout(t); resolve(m); });
      this.proc.stdin.write(JSON.stringify({ jsonrpc: "2.0", id, method, params }) + "\n");
    });
  }
  async init(name) {
    const r = await this.rpc("initialize", { protocolVersion: "2024-11-05", capabilities: {}, clientInfo: { name, version: "tpv" } });
    this.proc.stdin.write(JSON.stringify({ jsonrpc: "2.0", method: "notifications/initialized" }) + "\n");
    return r.result;
  }
  async tools() { return (await this.rpc("tools/list", {})).result.tools; }
  /** Returns { ok, text, json } — text is exactly what the server returned; json only if it parses. */
  async call(name, args = {}, timeoutMs) {
    const m = await this.rpc("tools/call", { name, arguments: args }, timeoutMs);
    if (m.error) return { ok: false, text: m.error.message, json: null };
    const text = m.result?.content?.[0]?.text ?? "";
    let json = null; try { json = JSON.parse(text); } catch { /* plain text */ }
    return { ok: !m.result?.isError, text, json };
  }
  close() { try { this.proc.stdin.end(); this.proc.kill(); } catch { /* already gone */ } }
}

/** Three agents sharing ONE context dir (a throwaway dir, so git-visible .ozone-context is not touched). */
export async function threeAgents({ linkRoots = "" } = {}) {
  const ctx = fs.mkdtempSync(path.join(os.tmpdir(), "ozone-mcp-ctx-"));
  const names = ["claude-code", "zcode", "third-agent"];
  const agents = {};
  for (const n of names) {
    const env = { OZONE_CONTEXT_DIR: ctx, OZONE_AGENT_NAME: n, ...(linkRoots ? { OZONE_MCP_LINK_ROOTS: linkRoots } : {}) };
    agents[n] = { shared: new McpClient(SERVERS.shared, env), ozone: new McpClient(SERVERS.ozone, env) };
    await agents[n].shared.init(n);
    await agents[n].ozone.init(n);
  }
  return { ctx, agents, close: () => Object.values(agents).forEach((a) => { a.shared.close(); a.ozone.close(); }) };
}

export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
export async function until(fn, { everyMs = 5000, maxMs = Number(process.env.OZONE_TPV_MAX_MS ?? 600000) } = {}) {
  const t0 = Date.now();
  for (;;) { const v = await fn(); if (v) return v; if (Date.now() - t0 > maxMs) return null; await sleep(everyMs); }
}
export const suite = (name) => baseSuite(`tpv-mcp-${name}`);
