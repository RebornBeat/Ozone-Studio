#!/usr/bin/env node
/**
 * Ozone-Studio agent connector — reference implementation (zero dependencies).
 *
 * Registers an agent with a running Ozone-Studio host, keeps the registration
 * alive, and pushes activity events to the monitor hub. This is the same wire
 * contract the browser plugin and any external agent use:
 *
 *   POST /pipelines/register   {pipeline_id, name, execute_url, roles?}
 *   POST /pipelines/unregister {pipeline_id}
 *   POST /monitor/activity     {kind, level, source, message}
 *   GET  /monitor/summary
 *
 * Roles: "agent" (receives task dispatch), "model" (serves pipeline-9 model
 * calls), "observer" (monitor feed only). This connector claims agent+model.
 *
 * The watch loop never gives up: if the host goes away it retries with
 * backoff and re-registers on recovery — a model connection must not drop
 * silently.
 *
 * Usage:
 *   node connect.js watch                resilient register + heartbeat loop (Ctrl-C leaves cleanly)
 *   node connect.js register             register once
 *   node connect.js activity "message"   push one activity event
 *   node connect.js unregister           leave the registry
 *
 * Environment:
 *   OZONE_HOST       host base URL          (default http://127.0.0.1:50051)
 *   ZCODE_AGENT_ID   registry id            (default 9002)
 *   ZCODE_AGENT_NAME agent name             (default zcode)
 *   ZCODE_ROLES      comma-separated roles  (default "agent,model")
 *   HEARTBEAT_SECS   heartbeat interval     (default 30)
 */

const host = process.env.OZONE_HOST || "http://127.0.0.1:50051";
const agentId = Number(process.env.ZCODE_AGENT_ID || 9002);
const agentName = process.env.ZCODE_AGENT_NAME || "zcode";
const roles = (process.env.ZCODE_ROLES || "agent,model")
  .split(",")
  .map((r) => r.trim())
  .filter(Boolean);
const heartbeatSecs = Number(process.env.HEARTBEAT_SECS || 30);

async function post(path, body) {
  const res = await fetch(`${host}${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body ?? {}),
  });
  if (!res.ok) throw new Error(`${path} → HTTP ${res.status}`);
  return res.json();
}

async function register() {
  // execute_url: where the host dispatches PipelineInputs to this agent.
  // The default points back at the host's own activity feed so pushed events
  // are observable; point it at your agent's real execute endpoint to
  // receive dispatched work.
  const out = await post("/pipelines/register", {
    pipeline_id: agentId,
    name: agentName,
    execute_url: `${host}/monitor/activity`,
    roles,
  });
  console.log(
    `[connect] registered ${agentName} (${agentId}) roles=[${roles.join(",")}]: ${JSON.stringify(out)}`,
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

async function watch() {
  let connected = false;
  let backoff = 1000;

  const ensureConnected = async () => {
    try {
      await register();
      if (!connected) {
        connected = true;
        backoff = 1000;
        await activity(
          `${agentName} connector live (roles: ${roles.join(", ")}, tools/zcode-connector)`,
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

  // Heartbeat: re-register on interval (idempotent, refreshes the entry).
  const heartbeat = setInterval(() => {
    if (connected) {
      register().catch((e) => {
        connected = false;
        console.error(`[heartbeat] host unreachable: ${e.message}`);
      });
    }
  }, heartbeatSecs * 1000);

  // Recovery loop: while disconnected, retry with capped backoff — the
  // connection comes back on its own when the host does.
  const recover = setInterval(async () => {
    if (!connected) {
      await ensureConnected().catch(() => {});
      backoff = Math.min(backoff * 2, 5000);
    }
  }, backoff);

  const leave = async () => {
    clearInterval(heartbeat);
    clearInterval(recover);
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
