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
 * >>> THE ONE THING YOU MUST FILL IN: handleModelCall() below. <<<
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
 *
 * Environment:
 *   OZONE_HOST         host base URL          (default http://127.0.0.1:50051)
 *   ZCODE_AGENT_ID      registry id            (default 9002 — see above; set 9 to become the default model)
 *   ZCODE_AGENT_NAME    agent name             (default zcode)
 *   ZCODE_ROLES         comma-separated roles  (default "agent,model")
 *   ZCODE_LOCAL_PORT    local server port      (default 9500)
 *   ZCODE_LOCAL_HOST    local server bind addr (default 127.0.0.1 — must be reachable from OZONE_HOST)
 *   HEARTBEAT_SECS      heartbeat interval     (default 30)
 */

const http = require("http");

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

async function post(path, body) {
  const res = await fetch(`${host}${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body ?? {}),
  });
  if (!res.ok) throw new Error(`${path} → HTTP ${res.status}`);
  return res.json();
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

  const heartbeat = setInterval(() => {
    if (connected) {
      register().catch((e) => {
        connected = false;
        console.error(`[heartbeat] host unreachable: ${e.message}`);
      });
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
  else {
    console.error(`unknown command: ${cmd} (watch | register | activity | unregister)`);
    process.exit(1);
  }
})().catch((e) => {
  console.error(`[error] ${e.message}`);
  process.exit(1);
});
