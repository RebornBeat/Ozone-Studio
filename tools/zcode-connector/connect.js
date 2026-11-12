#!/usr/bin/env node
/**
 * Ozone-Studio agent connector — reference implementation (zero dependencies).
 *
 * Registers an agent with a running Ozone-Studio host, keeps the registration
 * alive, pushes activity events to the monitor hub, AND (new) runs a real
 * local HTTP server implementing the pipeline-9 model-call wire contract —
 * so `execute_url` points at something that actually answers, not the old
 * `/monitor/activity` stub (which just logged and returned junk).
 *
 * Wire contract (matches src/pipeline/remote.rs's RemotePipelines::execute
 * and assets/pipelines/general/prompt/main.rs's PromptInput/PromptOutput):
 *
 *   POST <execute_url>   body: { data: {prompt, system_prompt?, max_tokens?,
 *                                        temperature?, action?, ...}, context: {...} }
 *                        response: flat JSON — { response, model_used,
 *                                  tokens_used?, finish_reason?, prompt_tokens? }
 *
 *   POST /pipelines/register   {pipeline_id, name, execute_url, roles?}
 *   POST /pipelines/unregister {pipeline_id}
 *   POST /monitor/activity     {kind, level, source, message}
 *   GET  /monitor/summary
 *
 * pipeline_id decides what this connector actually receives:
 *   - 9002 (default) — shows up as a present, heartbeating agent in the
 *     Monitor tab. It does NOT receive model calls: blueprint steps target
 *     pipeline_id 9 by default, and nothing routes to 9002 unless a future
 *     per-step model_override explicitly names it.
 *   - 9 — TAKES OVER pipeline 9, the default model tier. Every prompt call
 *     that doesn't carry a model_override.model_identifier resolving
 *     elsewhere comes here instead of the host's own configured backend
 *     (api/gguf/bitnet). Use this to make ZCode the actual default model.
 *   Both work today: src/grpc/mod.rs's register_remote_pipeline seeds the
 *   execution gate (PipelineRegistry.blueprints) for whatever id you pass,
 *   not just the compile-time 1-55 range.
 *
 * AUTH (2026-11-12): the host runs with OZONE_REQUIRE_SESSION on by default, and `/pipelines/register` (and /agent/*, /walk/*) need a
 * session. This connector authenticates with a device key (the same Ed25519 challenge as tools/ozone-cli): ZCODE_KEY_PATH, else
 * ~/.ozone-cli/agents/<ZCODE_AGENT_NAME>/device-key.pem when it exists, else the shared ~/.ozone-cli/device-key.pem. With no key it
 * still tries unauthenticated (a non-strict dev host) and says so. The key and the session token are never printed.
 *
 * JURISDICTION (phase 3): ZCode's model path is Z.ai (a third-country provider), so content that this connector forwards to it LEAVES
 * the machine. Before handleModelCall() runs, the connector asks the host (POST /jurisdiction/outbound/check, destination
 * ZCODE_PROVIDER_ENDPOINT) and does NOT forward a held or unjudgeable call. An unreachable host is not an allow.
 * ZCODE_JURISDICTION_CHECK=off is for a standalone connector with no host only.
 *
 * WHICH INTEGRATION TO USE: ZCode is a desktop app (no headless CLI), so handleModelCall() has nothing to spawn. The supported way for
 * ZCode to take part in Ozone is as an MCP CLIENT of tools/ozone-mcp (prompt via ozone_orchestrate, join walks, offer itself as an
 * agent provider with ozone_agent_offer). This connector remains the registry/heartbeat/activity reference and, in `watch`, tells the
 * monitor feed when agent-provider requests await ZCode. See docs/AGENT_INTEGRATION_ZCODE_CC.md.
 *
 * >>> THE ONE THING YOU MUST FILL IN (only for the pipeline-9 model role): handleModelCall() below. <<<
 * This file has no way to know how your ZCode installation actually runs
 * (a CLI subprocess? a local API server? something else) — that call is
 * yours to wire. Left unfilled, it responds honestly with an error instead
 * of fabricating a fake completion.
 *
 * Usage:
 *   node connect.js watch                resilient register + heartbeat + serve loop (Ctrl-C leaves cleanly)
 *   node connect.js register             register once (does not start the local server)
 *   node connect.js activity "message"   push one activity event
 *   node connect.js unregister           leave the registry
 *   node connect.js requests             agent-provider requests awaiting this agent's report
 *   node connect.js whoami               identity / key mode / jurisdiction check state (no secrets)
 *
 * Environment:
 *   OZONE_HOST         host base URL          (default http://127.0.0.1:50051)
 *   ZCODE_AGENT_ID      registry id            (default 9002 — see above; set 9 to become the default model)
 *   ZCODE_AGENT_NAME    agent name             (default zcode)
 *   ZCODE_ROLES         comma-separated roles  (default "agent,model")
 *   ZCODE_LOCAL_PORT    local server port      (default 9500)
 *   ZCODE_LOCAL_HOST    local server bind addr (default 127.0.0.1 — must be reachable from OZONE_HOST)
 *   HEARTBEAT_SECS      heartbeat interval     (default 30)
 *   ZCODE_KEY_PATH      device key for the session (default per-agent key if present, else ~/.ozone-cli/device-key.pem)
 *   ZCODE_PROVIDER_ENDPOINT  destination judged by the jurisdiction check (default https://api.z.ai/)
 *   ZCODE_JURISDICTION_CHECK off = skip the check (standalone only)
 */

const http = require("http");
const crypto = require("crypto");
const fs = require("fs");
const os = require("os");
const path = require("path");

const host = process.env.OZONE_HOST || "http://127.0.0.1:50051";
const agentId = Number(process.env.ZCODE_AGENT_ID || 9002);
const agentName = process.env.ZCODE_AGENT_NAME || "zcode";
const roles = (process.env.ZCODE_ROLES || "agent,model")
  .split(",")
  .map((r) => r.trim())
  .filter(Boolean);
const heartbeatSecs = Number(process.env.HEARTBEAT_SECS || 30);
const localPort = Number(process.env.ZCODE_LOCAL_PORT || 9500);
const localHost = process.env.ZCODE_LOCAL_HOST || "127.0.0.1";
const providerEndpoint = process.env.ZCODE_PROVIDER_ENDPOINT || "https://api.z.ai/";
const jurisdictionOff = ["off", "0", "false", "no"].includes(String(process.env.ZCODE_JURISDICTION_CHECK || "").toLowerCase());

// ── session (device-key challenge, same as tools/ozone-cli and tools/ozone-mcp) ───────────────────────────
const perAgentKey = path.join(os.homedir(), ".ozone-cli", "agents", agentName, "device-key.pem");
const keyPath =
  process.env.ZCODE_KEY_PATH ||
  (fs.existsSync(perAgentKey) ? perAgentKey : path.join(os.homedir(), ".ozone-cli", "device-key.pem"));
let session = null;
async function sessionToken(force = false) {
  if (session && !force) return session;
  let pem;
  try {
    pem = fs.readFileSync(keyPath, "utf8");
  } catch {
    return null; // no key: try unauthenticated (non-strict host); a strict host will answer 401 and say why
  }
  const priv = crypto.createPrivateKey(pem);
  const spki = crypto.createPublicKey(priv).export({ type: "spki", format: "der" });
  const pub = spki.subarray(spki.length - 32).toString("hex");
  const raw = async (p, body) => {
    const r = await fetch(`${host}${p}`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) });
    return r.json().catch(() => ({}));
  };
  const ch = await raw("/auth/challenge", { public_key: pub });
  if (!ch.challenge) throw new Error("host gave no auth challenge");
  const sig = crypto.sign(null, Buffer.from(ch.challenge, "hex"), priv).toString("hex");
  const au = await raw("/auth/authenticate", { public_key: pub, signature: sig });
  if (!au.session_token) throw new Error("authentication refused by the host");
  return (session = au.session_token);
}

// ============================================================================
// >>> FILL THIS IN <<< — the actual ZCode invocation.
// `data` is the pipeline-9 PromptInput shape: { prompt, system_prompt?,
// max_tokens?, temperature?, action?, model_override_config?, ... }.
// Must return (or resolve to) { response, model_used, tokens_used?,
// finish_reason?, prompt_tokens? } — the same contract pipeline 9's own
// execute_api/execute_bitnet return, so the orchestrator's token accounting
// and chat "handled by X" label work identically for ZCode.
// ============================================================================
async function handleModelCall(data) {
  throw new Error(
    "handleModelCall() is not wired to a real ZCode invocation yet — " +
      "edit tools/zcode-connector/connect.js and implement this function " +
      "(spawn your ZCode CLI, call its API, whatever your setup actually " +
      "is). This error is intentional: returning a fake completion here " +
      "would silently corrupt every token count and comparison test that " +
      "reads model_used.",
  );
}

// Strict hosts (OZONE_REQUIRE_SESSION default ON) refuse /pipelines/register, /pipelines/unregister and /monitor/activity without a credential.
// A connector on the same machine/user presents the host's internal token (file-permission boundary, same as the shared-context server):
// OZONE_INTERNAL_TOKEN, else OZONE_INTERNAL_TOKEN_FILE, else <OZONE_DATA_DIR|target/release/zsei_data|zsei_data>/.internal_token. Never printed.
function internalToken() {
  if (process.env.OZONE_INTERNAL_TOKEN) return process.env.OZONE_INTERNAL_TOKEN;
  const fs = require("fs");
  const path = require("path");
  const repo = path.resolve(__dirname, "..", "..");
  const files = [
    process.env.OZONE_INTERNAL_TOKEN_FILE,
    process.env.OZONE_DATA_DIR && path.join(process.env.OZONE_DATA_DIR, ".internal_token"),
    path.join(repo, "target", "release", "zsei_data", ".internal_token"),
    path.join(repo, "zsei_data", ".internal_token"),
  ].filter(Boolean);
  for (const f of files) {
    try {
      const v = fs.readFileSync(f, "utf8").trim();
      if (v) return v;
    } catch (_) { /* next */ }
  }
  return "";
}

async function call(method, p, body) {
  const once = async (force) => {
    const tok = await sessionToken(force);
    const headers = { "Content-Type": "application/json" };
    if (tok) headers.Authorization = `Bearer ${tok}`;
    return fetch(`${host}${p}`, { method, headers, body: method === "GET" ? undefined : JSON.stringify(body ?? {}) });
  };
  let res = await once(false);
  if (res.status === 401) res = await once(true); // session expired: re-authenticate once
  if (!res.ok) {
    const hint = res.status === 401 ? " (strict host: pair/authenticate with a device key — see docs/AGENT_INTEGRATION_ZCODE_CC.md §4)" : "";
    throw new Error(`${p} → HTTP ${res.status}${hint}`);
  }
  return res.json();
}
const post = (p, body) => call("POST", p, body);
const get = (p) => call("GET", p);

/** Ask the host whether this content may go to the model provider. Not an allow when the host cannot answer. */
async function clearOutbound(data) {
  if (jurisdictionOff) return { allow: true };
  let v;
  try {
    const input = { ...(data && typeof data === "object" ? data : {}), model_override_config: { api_endpoint: providerEndpoint, model_type: "api" } };
    const res = await fetch(`${host}/jurisdiction/outbound/check`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ pipeline_id: 9, input }),
    });
    v = await res.json();
  } catch (e) {
    return { allow: false, reason: `jurisdiction check unreachable (${e.message}) — the request was NOT forwarded (an unjudged call is never let through)` };
  }
  return v && v.allow === true ? { allow: true } : { allow: false, reason: (v && v.reason) || "AWAITING_OPERATOR: jurisdiction check did not clear this call" };
}

function executeUrl() {
  return `http://${localHost}:${localPort}/execute`;
}

async function register() {
  const out = await post("/pipelines/register", {
    pipeline_id: agentId,
    name: agentName,
    execute_url: executeUrl(),
    roles,
  });
  console.log(
    `[connect] registered ${agentName} (${agentId}) roles=[${roles.join(",")}] execute_url=${executeUrl()}: ${JSON.stringify(out)}`,
  );
}

async function activity(message, level = "info", kind = "agent") {
  const out = await post("/monitor/activity", {
    kind,
    level,
    source: agentName,
    message,
  });
  console.log(`[activity] ${JSON.stringify(out)}`);
}

async function unregister() {
  const out = await post("/pipelines/unregister", { pipeline_id: agentId });
  console.log(`[connect] unregistered: ${JSON.stringify(out)}`);
}

/** The actual model-call server — this is what makes execute_url real. */
function startLocalServer() {
  const server = http.createServer((req, res) => {
    if (req.method !== "POST" || req.url !== "/execute") {
      res.writeHead(404, { "Content-Type": "application/json" });
      res.end(JSON.stringify({ error: "not found" }));
      return;
    }
    let body = "";
    req.on("data", (chunk) => (body += chunk));
    req.on("end", async () => {
      const started = Date.now();
      let parsed;
      try {
        parsed = JSON.parse(body || "{}");
      } catch (e) {
        res.writeHead(400, { "Content-Type": "application/json" });
        res.end(JSON.stringify({ error: `invalid JSON: ${e.message}` }));
        return;
      }
      const data = parsed.data ?? parsed; // tolerate either shape
      const cleared = await clearOutbound(data);
      if (!cleared.allow) {
        res.writeHead(503, { "Content-Type": "application/json" });
        res.end(JSON.stringify({ error: cleared.reason, awaiting_operator: true }));
        activity(`call held by jurisdiction: ${String(cleared.reason).slice(0, 160)}`, "warn", "agent").catch(() => {});
        return;
      }
      try {
        const result = await handleModelCall(data);
        res.writeHead(200, { "Content-Type": "application/json" });
        res.end(JSON.stringify(result));
        activity(
          `call served in ${Date.now() - started}ms`,
          "ok",
          "agent",
        ).catch(() => {});
      } catch (e) {
        res.writeHead(500, { "Content-Type": "application/json" });
        res.end(JSON.stringify({ error: e.message || String(e) }));
        activity(`call failed: ${e.message}`, "error", "agent").catch(
          () => {},
        );
      }
    });
  });
  server.listen(localPort, localHost, () => {
    console.log(
      `[serve] listening on http://${localHost}:${localPort}/execute — POST here matches pipeline 9's exact contract`,
    );
  });
  return server;
}

async function watch() {
  let connected = false;
  let backoff = 1000;
  const localServer = startLocalServer();

  const ensureConnected = async () => {
    try {
      await register();
      if (!connected) {
        connected = true;
        backoff = 1000;
        await activity(
          `${agentName} connector live (roles: ${roles.join(", ")}, serving real calls on ${executeUrl()})`,
          "ok",
        );
      }
    } catch (e) {
      if (connected) {
        connected = false;
        console.error(`[watch] lost host: ${e.message} — retrying`);
      }
    }
  };

  await ensureConnected();

  let lastPending = 0;
  const heartbeat = setInterval(() => {
    if (connected) {
      register().catch((e) => {
        connected = false;
        console.error(`[heartbeat] host unreachable: ${e.message}`);
      });
      // Walk steps handed to THIS agent (agent-provider steps) wait for its report; say so on the monitor feed when the count changes.
      get(`/agent/requests?agent=${encodeURIComponent(agentName)}`)
        .then((r) => {
          const n = Array.isArray(r.requests) ? r.requests.length : 0;
          if (n !== lastPending) {
            lastPending = n;
            if (n > 0) activity(`${n} agent-provider request(s) await ${agentName}'s report — ozone_agent_requests, then ozone_agent_report`, "warn", "agent").catch(() => {});
          }
        })
        .catch(() => {});
    }
  }, heartbeatSecs * 1000);

  const recover = setInterval(async () => {
    if (!connected) {
      await ensureConnected().catch(() => {});
      backoff = Math.min(backoff * 2, 5000);
    }
  }, backoff);

  const leave = async () => {
    clearInterval(heartbeat);
    clearInterval(recover);
    localServer.close();
    try {
      await unregister();
    } catch {}
    process.exit(0);
  };
  process.on("SIGINT", leave);
  process.on("SIGTERM", leave);
}

const cmd = process.argv[2] || "watch";
const arg = process.argv[3];

(async () => {
  if (cmd === "watch") await watch();
  else if (cmd === "register") await register();
  else if (cmd === "activity") await activity(arg || "(empty)");
  else if (cmd === "unregister") await unregister();
  else if (cmd === "requests") console.log(JSON.stringify(await get(`/agent/requests?agent=${encodeURIComponent(agentName)}`), null, 2));
  else if (cmd === "whoami") console.log(JSON.stringify({ agent: agentName, host, key_path: fs.existsSync(keyPath) ? keyPath : null, per_agent_key: keyPath === perAgentKey, provider_endpoint: providerEndpoint, jurisdiction_check: jurisdictionOff ? "off" : "on" }));
  else {
    console.error(`unknown command: ${cmd} (watch | register | activity | unregister | requests | whoami)`);
    process.exit(1);
  }
})().catch((e) => {
  console.error(`[error] ${e.message}`);
  process.exit(1);
});
