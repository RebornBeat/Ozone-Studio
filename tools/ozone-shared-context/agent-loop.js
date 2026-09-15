#!/usr/bin/env node
/**
 * Agent presence loop — ZCode (or any agent) hooked via the ACTUAL MCP.
 *
 * Every action is a real MCP tools/call against ozone-shared-context: the
 * host meters each one in the usage ledger and mirrors it to the monitor
 * feed. Run it and watch the calls flow; stop it any time.
 *
 *   node agent-loop.js                     # heartbeat every 4 min, forever
 *   INTERVAL_SECS=60 node agent-loop.js    # faster for observing
 *
 * Env: OZONE_AGENT_NAME (default zcode), OZONE_TASK, OZONE_FILES (comma-sep)
 */

const { spawn } = require("child_process");
const path = require("path");

const SERVER = path.join(__dirname, "server.js");
const AGENT = process.env.OZONE_AGENT_NAME || "zcode";
const TASK = process.env.OZONE_TASK || "standing by on the Ozone-Studio connection";
const FILES = (process.env.OZONE_FILES || "").split(",").filter(Boolean);
const INTERVAL = Number(process.env.INTERVAL_SECS || 240) * 1000;

function mcpCall(name, args) {
  return new Promise((resolve, reject) => {
    const child = spawn("node", [SERVER], { stdio: ["pipe", "pipe", "inherit"] });
    let out = "";
    child.stdout.setEncoding("utf8");
    child.stdout.on("data", (d) => {
      out += d;
      const lines = out.split("\n").filter(Boolean);
      for (const line of lines) {
        try {
          const m = JSON.parse(line);
          if (m.id === 2 && m.result) {
            child.kill();
            resolve(JSON.parse(m.result.content[0].text));
            return;
          }
          if (m.id === 2 && m.error) {
            child.kill();
            reject(new Error(m.error.message));
            return;
          }
        } catch {}
      }
    });
    child.stdin.write(JSON.stringify({ jsonrpc: "2.0", id: 1, method: "initialize", params: { protocolVersion: "2024-11-05" } }) + "\n");
    child.stdin.write(JSON.stringify({ jsonrpc: "2.0", id: 2, method: "tools/call", params: { name, arguments: args } }) + "\n");
    child.stdin.end();
    setTimeout(() => { child.kill(); reject(new Error("timeout")); }, 15000).unref?.();
  });
}

async function tick() {
  const ts = new Date().toLocaleTimeString();
  try {
    await mcpCall("presence_heartbeat", { agent: AGENT, role: "agent+model", current_files: FILES, task: TASK });
    process.stdout.write(`[${ts}] heartbeat ok\n`);
  } catch (e) {
    process.stdout.write(`[${ts}] heartbeat: ${e.message}\n`);
  }
}

(async () => {
  // ONE_SHOT=1: claim + single heartbeat, then exit — for external schedulers
  // (cron/automations) that call this repeatedly.
  if (process.env.ONE_SHOT === "1") {
    if (FILES.length > 0) {
      try {
        await mcpCall("file_claim", { agent: AGENT, files: FILES, reason: "active workspace" });
      } catch (e) {
        process.stdout.write(`[${new Date().toLocaleTimeString()}] claim: ${e.message}\n`);
      }
    }
    await tick();
    return;
  }
  process.stdout.write(`[agent-loop] ${AGENT} hooked via MCP — heartbeat every ${INTERVAL / 1000}s (Ctrl-C to stop)\n`);
  if (FILES.length > 0) {
    try {
      const r = await mcpCall("file_claim", { agent: AGENT, files: FILES, reason: "active workspace" });
      process.stdout.write(`[agent-loop] claim: ${JSON.stringify(r)}\n`);
    } catch (e) {
      process.stdout.write(`[agent-loop] claim: ${e.message}\n`);
    }
  }
  await tick();
  setInterval(tick, INTERVAL);
})();
