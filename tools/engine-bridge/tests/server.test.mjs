import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { once } from "node:events";
import net from "node:net";
import { resolve, dirname } from "node:path";
import { createInterface } from "node:readline";
import test from "node:test";
import { setTimeout as delay } from "node:timers/promises";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const projectRoot = resolve(__dirname, "..");
const entryPath = resolve(projectRoot, "dist/index.js");

function createClient() {
  const port = 3200 + Math.floor(Math.random() * 500) + parseInt(randomUUID().slice(0, 2), 16);
  const child = spawn("node", [entryPath], {
    cwd: projectRoot,
    env: {
      ...process.env,
      ROBLOX_LUAU_BRIDGE_PORT: String(port)
    },
    stdio: ["pipe", "pipe", "pipe"]
  });
  const pending = new Map();
  let nextId = 1;

  const stdout = createInterface({ input: child.stdout });
  stdout.on("line", (line) => {
    if (!line.trim()) {
      return;
    }

    const message = JSON.parse(line);
    if (typeof message.id !== "number") {
      return;
    }

    const entry = pending.get(message.id);
    if (!entry) {
      return;
    }

    pending.delete(message.id);
    entry.resolve(message);
  });

  const failPending = (error) => {
    for (const entry of pending.values()) {
      entry.reject(error);
    }
    pending.clear();
  };

  child.once("error", failPending);
  child.once("exit", (code, signal) => {
    failPending(new Error(`MCP child exited before response (code=${code}, signal=${signal})`));
  });

  return {
    port,
    async initialize() {
      const response = await this.request("initialize", {
        protocolVersion: "2024-11-05",
        capabilities: {},
        clientInfo: {
          name: "node-test",
          version: "0.1.0"
        }
      });
      this.notify("notifications/initialized", {});
      return response;
    },
    request(method, params = {}) {
      return new Promise((resolvePromise, rejectPromise) => {
        const id = nextId++;
        pending.set(id, {
          resolve: resolvePromise,
          reject: rejectPromise
        });
        child.stdin.write(`${JSON.stringify({ jsonrpc: "2.0", id, method, params })}\n`);
      });
    },
    notify(method, params = {}) {
      child.stdin.write(`${JSON.stringify({ jsonrpc: "2.0", method, params })}\n`);
    },
    async stop() {
      stdout.close();
      child.kill("SIGTERM");
      const exited = once(child, "exit");
      await Promise.race([
        exited,
        delay(3_000).then(() => {
          child.kill("SIGKILL");
          return exited;
        })
      ]);
    }
  };
}

function requestWithoutHostHeader(port, path) {
  return new Promise((resolvePromise, rejectPromise) => {
    const socket = net.createConnection({
      host: "127.0.0.1",
      port
    });
    const chunks = [];

    socket.on("connect", () => {
      socket.write(`GET ${path} HTTP/1.0\r\nConnection: close\r\n\r\n`);
    });
    socket.on("data", (chunk) => {
      chunks.push(chunk);
    });
    socket.on("end", () => {
      try {
        const rawResponse = Buffer.concat(chunks).toString("utf8");
        const [head, body = ""] = rawResponse.split("\r\n\r\n");
        const statusLine = head.split("\r\n")[0] || "";
        const statusCode = Number(statusLine.split(" ")[1]);
        resolvePromise({
          statusCode,
          body: body.trim()
        });
      } catch (error) {
        rejectPromise(error);
      }
    });
    socket.on("error", rejectPromise);
  });
}

test("tools/list exposes the expected MCP tools", async () => {
  const client = createClient();
  try {
    const initializeResponse = await client.initialize();
    assert.equal(initializeResponse.result.serverInfo.name, "gamedev-all-in-one");

    const response = await client.request("tools/list");
    const toolNames = response.result.tools.map((tool) => tool.name).sort();

    assert.deepEqual(toolNames, [
      "doctor",
      "inspect_project",
      "list_capabilities",
      "project_init",
      "roblox_create_workspace_part",
      "roblox_run_code"
    ]);
  } finally {
    await client.stop();
  }
});

test("doctor returns a structured summary payload", async () => {
  const client = createClient();
  try {
    await client.initialize();

    const response = await client.request("tools/call", {
      name: "doctor",
      arguments: {
        includeManifest: true
      }
    });
    const summary = response.result.structuredContent.summary;

    assert.equal(summary.manifestChecked, true);
    assert.equal(typeof summary.ok, "boolean");
    assert.equal(Array.isArray(summary.missing), true);
  } finally {
    await client.stop();
  }
});

test("bridge rejects an empty handshake body", async () => {
  const client = createClient();
  try {
    await client.initialize();

    const response = await fetch(`http://127.0.0.1:${client.port}/runtime/handshake`, {
      method: "POST"
    });
    const body = await response.json();

    assert.equal(response.status, 400);
    assert.equal(body.error, "Empty request body");
  } finally {
    await client.stop();
  }
});

test("bridge rejects a request without a host header", async () => {
  const client = createClient();
  try {
    await client.initialize();

    const response = await requestWithoutHostHeader(client.port, "/runtime/health");

    assert.equal(response.statusCode, 400);
    assert.equal(JSON.parse(response.body).error, "Missing host header");
  } finally {
    await client.stop();
  }
});

test("bridge rejects an invalid timeout query parameter", async () => {
  const client = createClient();
  try {
    await client.initialize();

    const response = await fetch(`http://127.0.0.1:${client.port}/runtime/commands/next?timeoutMs=bad`, {
      method: "GET"
    });
    const body = await response.json();

    assert.equal(response.status, 400);
    assert.equal(body.error, "Invalid timeoutMs query parameter");
  } finally {
    await client.stop();
  }
});

test("bridge rejects an invalid command result body", async () => {
  const client = createClient();
  try {
    await client.initialize();

    const response = await fetch(`http://127.0.0.1:${client.port}/runtime/commands/result`, {
      method: "POST",
      headers: {
        "content-type": "application/json"
      },
      body: JSON.stringify({
        requestId: "not-a-uuid",
        status: "ok",
        completedAt: new Date().toISOString()
      })
    });
    const body = await response.json();

    assert.equal(response.status, 400);
    assert.equal(body.error, "Invalid request body");
  } finally {
    await client.stop();
  }
});
