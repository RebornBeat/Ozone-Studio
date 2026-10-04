#!/usr/bin/env node
// Ozone-Studio CLI — a real command-line client for the host, per operator
// directive 2026-09-29 ("I now also want to create a mobile app and cli").
// Same real Ed25519 device-identity + session-token pattern proven
// throughout this session (main.js's ensureSessionToken, this session's
// own verify_*.mjs scripts) — a persisted device keypair under
// ~/.ozone-cli/, not a throwaway key per invocation (that would register
// a new device identity on every single command).
//
// Usage:
//   ozone status                              — real /health
//   ozone order [--due today|overdue|week]    — real /order/global
//   ozone capture "<text>" [--kind todo|note|meeting] [--due <ISO date>]
//                                              — real /task/create
//   ozone chat "<prompt>"                     — real /orchestrate (blocking,
//                                                same 14-stage flow the UI uses)
//   ozone mcp list                            — real /mcp/tools
//   ozone mcp call <tool> '<json input>'      — real full gated /mcp/call
//
// Env: OZONE_HOST (default http://127.0.0.1:50051)

import crypto from "node:crypto";
import http from "node:http";
import https from "node:https";
import fs from "node:fs";
import path from "node:path";
import os from "node:os";

const OZONE_HOST = process.env.OZONE_HOST ?? "http://127.0.0.1:50051";
const CONFIG_DIR = path.join(os.homedir(), ".ozone-cli");
const KEY_PATH = path.join(CONFIG_DIR, "device-key.pem");

function req(method, urlPath, body) {
  return new Promise((resolve, reject) => {
    const url = new URL(urlPath, OZONE_HOST);
    const data = body ? JSON.stringify(body) : null;
    const transport = url.protocol === "https:" ? https : http;
    const r = transport.request(
      { hostname: url.hostname, port: url.port, path: url.pathname + url.search, method, headers: { "Content-Type": "application/json" }, timeout: 300000 },
      (res) => {
        let d = "";
        res.on("data", (c) => (d += c));
        res.on("end", () => {
          try { resolve(JSON.parse(d)); } catch { resolve(d); }
        });
      },
    );
    r.on("error", reject);
    r.on("timeout", () => { r.destroy(); reject(new Error("request timed out")); });
    if (data) r.write(data);
    r.end();
  });
}

let cachedToken = null;

async function ensureSessionToken() {
  if (cachedToken) return cachedToken;
  fs.mkdirSync(CONFIG_DIR, { recursive: true });
  let privateKeyPem;
  if (fs.existsSync(KEY_PATH)) {
    privateKeyPem = fs.readFileSync(KEY_PATH, "utf8");
  } else {
    const kp = crypto.generateKeyPairSync("ed25519");
    privateKeyPem = kp.privateKey.export({ type: "pkcs8", format: "pem" });
    fs.writeFileSync(KEY_PATH, privateKeyPem, { mode: 0o600 });
  }
  const privateKey = crypto.createPrivateKey(privateKeyPem);
  const spki = crypto.createPublicKey(privateKey).export({ type: "spki", format: "der" });
  const rawPub = spki.subarray(spki.length - 32).toString("hex");

  const ch = await req("POST", "/auth/challenge", { public_key: rawPub });
  if (!ch?.challenge) throw new Error("no challenge from host — is it running?");
  const signature = crypto.sign(null, Buffer.from(ch.challenge, "hex"), privateKey).toString("hex");
  const auth = await req("POST", "/auth/authenticate", { public_key: rawPub, signature });
  if (!auth?.success || !auth?.session_token) throw new Error(`host auth failed: ${auth?.error ?? "unknown"}`);
  cachedToken = auth.session_token;
  return cachedToken;
}

function fmtDate(ts) {
  if (ts == null) return "-";
  return new Date(ts * 1000).toLocaleString();
}

function printTable(rows, cols) {
  if (rows.length === 0) { console.log("  (none)"); return; }
  const widths = cols.map((c) => Math.max(c.label.length, ...rows.map((r) => String(c.get(r) ?? "").length)));
  console.log("  " + cols.map((c, i) => c.label.padEnd(widths[i])).join("  "));
  console.log("  " + widths.map((w) => "-".repeat(w)).join("  "));
  for (const r of rows) {
    console.log("  " + cols.map((c, i) => String(c.get(r) ?? "").padEnd(widths[i])).join("  "));
  }
}

async function cmdStatus() {
  const health = await req("GET", "/health");
  console.log(`Ozone-Studio ${health.version ?? "?"} — ${health.healthy ? "healthy" : "unreachable"}`);
  console.log(`  uptime: ${health.uptime_secs ?? "?"}s   active tasks: ${health.active_tasks ?? 0}   peers: ${health.peer_count ?? 0}`);
}

async function cmdOrder(args) {
  const dueArg = args.includes("--due") ? args[args.indexOf("--due") + 1] : null;
  const q = dueArg ? `?due=${encodeURIComponent(dueArg)}` : "";
  const order = await req("GET", `/order/global${q}`);
  for (const bucket of ["live", "queued", "paused", "interrupted"]) {
    const items = order[bucket] ?? [];
    if (items.length === 0) continue;
    console.log(`\n${bucket.toUpperCase()} (${items.length})`);
    printTable(items, [
      { label: "id", get: (r) => r.task_id },
      { label: "kind", get: (r) => r.kind },
      { label: "name", get: (r) => (r.name ?? "").slice(0, 60) },
      { label: "due", get: (r) => fmtDate(r.due_at) },
    ]);
  }
  if (dueArg) {
    console.log(`\n${dueArg.toUpperCase()}`);
    const bucketItems = [...(order.done ?? []), ...(order.other ?? []), ...(order.live ?? []), ...(order.queued ?? []), ...(order.paused ?? []), ...(order.interrupted ?? [])];
    printTable(bucketItems, [
      { label: "id", get: (r) => r.task_id },
      { label: "kind", get: (r) => r.kind },
      { label: "name", get: (r) => (r.name ?? "").slice(0, 60) },
      { label: "due", get: (r) => fmtDate(r.due_at) },
    ]);
  }
}

async function cmdCapture(args) {
  const text = args.find((a) => !a.startsWith("--"));
  if (!text) { console.error("usage: ozone capture \"<text>\" [--kind todo|note|meeting] [--due <ISO date>]"); process.exit(1); }
  const kind = args.includes("--kind") ? args[args.indexOf("--kind") + 1] : "todo";
  const dueArg = args.includes("--due") ? args[args.indexOf("--due") + 1] : null;
  const due_at = dueArg ? Math.floor(new Date(dueArg).getTime() / 1000) : undefined;
  const result = await req("POST", "/task/create", {
    prompt: text, session_token: "", created_by: "cli", kind, due_at,
    note_body: kind === "note" ? text : undefined,
  });
  if (result.success) console.log(`Captured — task ${result.task_id}`);
  else console.error(`Failed: ${result.error ?? "unknown error"}`);
}

async function cmdChat(args) {
  const prompt = args.join(" ");
  if (!prompt.trim()) { console.error('usage: ozone chat "<prompt>"'); process.exit(1); }
  console.log("Thinking (this runs the full orchestration pipeline, can take a while)...");
  const result = await req("POST", "/orchestrate", {
    prompt, user_id: 0, device_id: 0, consciousness_enabled: true, // the consciousness gate participates (config [consciousness].enabled gates the loops; this is the per-request gate) token_budget: 100000,
  });
  if (result.response) console.log(`\n${result.response}\n`);
  else console.error(`No response: ${result.error ?? "unknown"}`);
}

async function cmdMcpList() {
  const result = await req("GET", "/mcp/tools");
  printTable(result.tools ?? [], [
    { label: "name", get: (r) => r.name },
    { label: "version", get: (r) => r.server_version ?? "-" },
    { label: "endpoint", get: (r) => r.endpoint },
  ]);
}

async function cmdMcpCall(args) {
  const [tool, inputJson] = args;
  if (!tool) { console.error("usage: ozone mcp call <tool> '<json input>'"); process.exit(1); }
  let input = {};
  try { input = inputJson ? JSON.parse(inputJson) : {}; } catch { console.error("input must be valid JSON"); process.exit(1); }
  const token = await ensureSessionToken();
  const result = await req("POST", "/mcp/call", {
    tool, agent: "ozone-cli", input, context: { session_token: token },
  });
  console.log(JSON.stringify(result, null, 2));
}

async function main() {
  const [, , cmd, ...rest] = process.argv;
  try {
    switch (cmd) {
      case "status": await cmdStatus(); break;
      case "order": await cmdOrder(rest); break;
      case "capture": await cmdCapture(rest); break;
      case "chat": await cmdChat(rest); break;
      case "mcp":
        if (rest[0] === "list") await cmdMcpList();
        else if (rest[0] === "call") await cmdMcpCall(rest.slice(1));
        else { console.error("usage: ozone mcp list | ozone mcp call <tool> '<json>'"); process.exit(1); }
        break;
      default:
        console.log(
          "Ozone-Studio CLI\n\n" +
          "  ozone status\n" +
          "  ozone order [--due today|overdue|week|upcoming]\n" +
          '  ozone capture "<text>" [--kind todo|note|meeting] [--due <ISO date>]\n' +
          '  ozone chat "<prompt>"\n' +
          "  ozone mcp list\n" +
          "  ozone mcp call <tool> '<json input>'\n",
        );
        process.exit(cmd ? 1 : 0);
    }
  } catch (e) {
    console.error(`Error: ${e instanceof Error ? e.message : String(e)}`);
    process.exit(1);
  }
}

main();
