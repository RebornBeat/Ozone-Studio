// Shared E2E harness (2026-11-03). Real host, real device-key auth (same pattern as tools/ozone-cli),
// no mocks. Every suite prints PASS/FAIL/SKIP rows and appends them to
// docs/evidence/test-runs/<date>-<suite>.md so a suite is green only with its evidence file.
import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const HOST = process.env.OZONE_HOST ?? "http://127.0.0.1:50051";
const HERE = path.dirname(fileURLToPath(import.meta.url));
const EVIDENCE_DIR = path.resolve(HERE, "../../docs/evidence/test-runs");
const KEY_PATH = path.join(os.homedir(), ".ozone-cli", "device-key.pem");

export async function http(method, p, body, headers = {}, timeoutMs = 120000) {
  // Strict hosts (default ON) gate protected route families on `Authorization: Bearer <session>`: once a device session exists, every call
  // carries it (the auth handshake itself and /health never need one).
  if (session && !headers.authorization && !headers.Authorization && !p.startsWith("/auth/") && p !== "/health") {
    headers = { ...headers, authorization: `Bearer ${session}` };
  }
  const ctl = new AbortController();
  const t = setTimeout(() => ctl.abort(), timeoutMs);
  try {
    const r = await fetch(HOST + p, {
      method,
      headers: { "content-type": "application/json", ...headers },
      body: body === undefined ? undefined : JSON.stringify(body),
      signal: ctl.signal,
    });
    const text = await r.text();
    let json; try { json = JSON.parse(text); } catch { json = text; }
    return { status: r.status, json };
  } finally { clearTimeout(t); }
}

let session = null;
export async function sessionToken() {
  if (session) return session;
  fs.mkdirSync(path.dirname(KEY_PATH), { recursive: true });
  let pem;
  if (fs.existsSync(KEY_PATH)) pem = fs.readFileSync(KEY_PATH, "utf8");
  else {
    const kp = crypto.generateKeyPairSync("ed25519");
    pem = kp.privateKey.export({ type: "pkcs8", format: "pem" });
    fs.writeFileSync(KEY_PATH, pem, { mode: 0o600 });
  }
  const priv = crypto.createPrivateKey(pem);
  const spki = crypto.createPublicKey(priv).export({ type: "spki", format: "der" });
  const pub = spki.subarray(spki.length - 32).toString("hex");
  const ch = (await http("POST", "/auth/challenge", { public_key: pub })).json;
  const sig = crypto.sign(null, Buffer.from(ch.challenge, "hex"), priv).toString("hex");
  const au = (await http("POST", "/auth/authenticate", { public_key: pub, signature: sig })).json;
  if (!au?.session_token) throw new Error("auth failed: " + JSON.stringify(au));
  return (session = au.session_token);
}

/** Run a pipeline through /pipeline/execute with a real session. Returns the output data object. */
export async function pipeline(id, data, ctx = {}) {
  const tok = await sessionToken();
  const r = (await http("POST", "/pipeline/execute", {
    pipeline_id: id,
    input: { data, context: { user_id: 1, device_id: 1, workspace_id: ctx.workspace_id ?? null, project_id: ctx.project_id ?? null, task_context_id: null, metadata: {} } },
    session_token: tok,
  })).json;
  return r;
}

export async function mcp(tool, input = {}, agent = "e2e") {
  await sessionToken();
  return (await http("POST", "/mcp/call", { tool, agent, input })).json;
}

export async function orchestrate(body, { pollMs = 15000, maxMs = 3600000 } = {}) {
  // /orchestrate REQUIRES a credential on strict hosts: a real device session (body token + Bearer header).
  const sess = await sessionToken();
  const ack = (await http("POST", "/orchestrate", { user_id: 1, device_id: 1, consciousness_enabled: false, token_budget: 100000, session_token: sess, ...body, submit_only: true })).json;
  const tok = ack.run_token;
  const t0 = Date.now();
  for (;;) {
    const s = (await http("GET", `/orchestrate/status?run_token=${tok}`)).json;
    if (s.state && s.state !== "running") return s;
    if (Date.now() - t0 > maxMs) return { state: "timeout", run_token: tok };
    await new Promise((r) => setTimeout(r, pollMs));
  }
}

export function suite(name) {
  const rows = [];
  const date = new Date().toISOString().slice(0, 10);
  const t0 = Date.now();
  return {
    pass(id, what, evidence = "") { rows.push(["PASS", id, what, evidence]); console.log(`PASS  ${id}  ${what}${evidence ? "  — " + String(evidence).slice(0, 140) : ""}`); },
    fail(id, what, evidence = "") { rows.push(["FAIL", id, what, evidence]); console.log(`FAIL  ${id}  ${what}${evidence ? "  — " + String(evidence).slice(0, 220) : ""}`); },
    skip(id, what, why) { rows.push(["SKIP", id, what, why]); console.log(`SKIP  ${id}  ${what}  (${why})`); },
    check(id, what, ok, evidence = "") { ok ? this.pass(id, what, evidence) : this.fail(id, what, evidence); },
    finish() {
      const c = (k) => rows.filter((r) => r[0] === k).length;
      const md = `# ${name} — ${date}\n\nhost: ${HOST} · duration ${Math.round((Date.now() - t0) / 1000)}s · PASS ${c("PASS")} · FAIL ${c("FAIL")} · SKIP ${c("SKIP")}\n\n| result | id | test | evidence |\n|---|---|---|---|\n` +
        rows.map((r) => `| ${r[0]} | ${r[1]} | ${r[2]} | ${String(r[3]).replace(/\|/g, "\\|").replace(/\n/g, " ").slice(0, 300)} |`).join("\n") + "\n";
      fs.mkdirSync(EVIDENCE_DIR, { recursive: true });
      const file = path.join(EVIDENCE_DIR, `${date}-${name}.md`);
      fs.writeFileSync(file, md);
      console.log(`\n${name}: PASS ${c("PASS")} FAIL ${c("FAIL")} SKIP ${c("SKIP")} → ${file}`);
      return c("FAIL");
    },
  };
}
