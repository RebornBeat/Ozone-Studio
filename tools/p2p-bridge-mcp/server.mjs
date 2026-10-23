// Ozone-Studio P2P Bridge MCP (2026-10-21, operator directive).
//
// WHY THIS EXISTS: this session proved real P2P sync between two
// Ozone-Studio instances in two jurisdictions (a local machine + a
// Contabo EU VPS), but every single test required a hand-run sequence:
// open an SSH tunnel forwarding the remote's HTTP+P2P ports, read the
// remote's peer id out of its log by eye, hand-build a multiaddr, then
// POST /network/connect-peer. That manual sequence is exactly what this
// MCP automates into one call — not a new transport, not a replacement
// for native P2P (see docs/P2P_AND_SHARED_MCP_COMMUNICATION_GUIDE.md §5.5):
// this tool only establishes/tears down the SSH bridge and drives the
// REAL, already-existing Ozone-Studio HTTP routes once it's up.
//
// SECURITY MODEL (deliberate, matching terminal-mcp's "unconfigured =
// LOCKED" doctrine, not an afterthought):
// 1. ALLOWED HOSTS REQUIRED — OZONE_P2P_BRIDGE_ALLOWED_HOSTS
//    (comma-separated hostnames/IPs). If unset/empty, p2p_bridge_connect
//    refuses every host — this MCP never boots able to bridge to an
//    arbitrary, caller-supplied address. Unlike terminal-mcp's broad
//    "what command" risk, this tool's specific risk is "autonomously
//    connects a P2P peer to whatever host it's told to" — bounding that
//    to a pre-approved list is the first and primary gate.
// 2. The remote SSH connection uses BatchMode (no password fallback,
//    key-only) and StrictHostKeyChecking=accept-new (TOFU, not
//    no-checking) — never silently disables host-key verification.
// 3. This tool bridges the TUNNEL and calls the real connect-peer route;
//    it never touches the trust gate (approve-peer/paired_peers) — a
//    connected-but-unpaired peer stays exactly that, which is the real
//    Ozone-Studio host's job to decide, not this tool's.
// 4. Every call arrives through /mcp/call, so agent identity/metering is
//    enforced host-side — this process never trusts its caller beyond
//    the allowed-hosts check, same posture as terminal-mcp.
//
// Endpoints: POST /call {tool, agent, input}
//            GET  /status
// Tools: p2p_bridge_connect {remote_host, ssh_user?, ssh_key_path?,
//          remote_http_port?, remote_p2p_port?}
//        p2p_bridge_disconnect {remote_host}
//        p2p_bridge_status {}
// Env:   OZONE_P2P_BRIDGE_PORT (default 3278)
//        OZONE_P2P_BRIDGE_ALLOWED_HOSTS (comma-separated, required)
//        OZONE_P2P_BRIDGE_SSH_KEY (default ssh key path, overridable per call)
//        OZONE_HOST (default http://127.0.0.1:50051 — same convention
//          code-mcp/re-mcp/zoom-mcp already use)

import { createServer } from "node:http";
import { spawn } from "node:child_process";
import net from "node:net";
import http from "node:http";

const PORT = Number(process.env.OZONE_P2P_BRIDGE_PORT ?? 3278);
const ALLOWED_HOSTS = (process.env.OZONE_P2P_BRIDGE_ALLOWED_HOSTS ?? "")
  .split(",")
  .map((s) => s.trim())
  .filter(Boolean);
const DEFAULT_SSH_KEY = process.env.OZONE_P2P_BRIDGE_SSH_KEY ?? "";
const OZONE_HOST = process.env.OZONE_HOST ?? "http://127.0.0.1:50051";
const startedAt = Date.now();

/// Active bridges, keyed by remote_host. Each: { child, localHttpPort,
/// localP2pPort, remotePeerId, sshUser, startedAt }.
const bridges = new Map();

function reply(res, code, body) {
  res.writeHead(code, { "Content-Type": "application/json" });
  res.end(JSON.stringify(body));
}

function httpGetJson(url, timeoutMs) {
  return new Promise((resolve, reject) => {
    const req = http.get(url, { timeout: timeoutMs }, (res) => {
      let data = "";
      res.on("data", (c) => (data += c));
      res.on("end", () => {
        try { resolve(JSON.parse(data)); } catch (e) { reject(e); }
      });
    });
    req.on("timeout", () => { req.destroy(); reject(new Error("timeout")); });
    req.on("error", reject);
  });
}

function httpPostJson(url, body, timeoutMs) {
  return new Promise((resolve, reject) => {
    const data = JSON.stringify(body);
    const u = new URL(url);
    const req = http.request(
      { hostname: u.hostname, port: u.port, path: u.pathname, method: "POST",
        headers: { "Content-Type": "application/json", "Content-Length": Buffer.byteLength(data) },
        timeout: timeoutMs },
      (res) => {
        let out = "";
        res.on("data", (c) => (out += c));
        res.on("end", () => { try { resolve(JSON.parse(out)); } catch (e) { reject(e); } });
      }
    );
    req.on("timeout", () => { req.destroy(); reject(new Error("timeout")); });
    req.on("error", reject);
    req.write(data);
    req.end();
  });
}

/// Ask the OS for a free local port (bind to 0, read it back, release).
/// A brief TOCTOU race is possible but standard/acceptable for this kind
/// of ephemeral allocation — ssh itself will fail loudly if it loses the
/// race, not silently misbehave.
function getFreePort() {
  return new Promise((resolve, reject) => {
    const srv = net.createServer();
    srv.listen(0, "127.0.0.1", () => {
      const port = srv.address().port;
      srv.close(() => resolve(port));
    });
    srv.on("error", reject);
  });
}

async function waitForTunnel(localHttpPort, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const health = await httpGetJson(`http://127.0.0.1:${localHttpPort}/health`, 2000);
      if (health && health.healthy) return health;
    } catch { /* tunnel/remote not ready yet, keep polling */ }
    await new Promise((r) => setTimeout(r, 300));
  }
  return null;
}

async function bridgeConnect(input) {
  const remoteHost = String(input.remote_host ?? "");
  if (!ALLOWED_HOSTS.length) {
    return { success: false, error: "no OZONE_P2P_BRIDGE_ALLOWED_HOSTS configured — p2p_bridge_connect is locked (unconfigured = locked, by design)" };
  }
  if (!ALLOWED_HOSTS.includes(remoteHost)) {
    return { success: false, error: `host '${remoteHost}' is not in OZONE_P2P_BRIDGE_ALLOWED_HOSTS (${ALLOWED_HOSTS.join(", ")})` };
  }
  const sshUser = String(input.ssh_user ?? "root");
  const sshKeyPath = String(input.ssh_key_path ?? DEFAULT_SSH_KEY ?? "");
  if (!sshKeyPath) {
    return { success: false, error: "no ssh_key_path given and no OZONE_P2P_BRIDGE_SSH_KEY configured" };
  }
  const remoteHttpPort = Number(input.remote_http_port ?? 50051);
  const remoteP2pPort = Number(input.remote_p2p_port ?? 9090);

  // Idempotent: a live existing bridge to this host is reused, not duplicated.
  const existing = bridges.get(remoteHost);
  if (existing && existing.child.exitCode === null && !existing.child.killed) {
    return {
      success: true,
      reused_existing_bridge: true,
      remote_host: remoteHost,
      remote_peer_id: existing.remotePeerId,
      local_http_port: existing.localHttpPort,
      local_p2p_port: existing.localP2pPort,
    };
  }

  const localHttpPort = await getFreePort();
  const localP2pPort = await getFreePort();

  const sshArgs = [
    "-N",
    "-o", "BatchMode=yes",
    "-o", "ConnectTimeout=8",
    "-o", "ExitOnForwardFailure=yes",
    "-o", "ServerAliveInterval=30",
    "-o", "ServerAliveCountMax=3",
    "-o", "StrictHostKeyChecking=accept-new",
    "-L", `${localHttpPort}:127.0.0.1:${remoteHttpPort}`,
    "-L", `${localP2pPort}:127.0.0.1:${remoteP2pPort}`,
    "-i", sshKeyPath,
    `${sshUser}@${remoteHost}`,
  ];
  const child = spawn("ssh", sshArgs, { stdio: ["ignore", "pipe", "pipe"] });
  let sshStderr = "";
  child.stderr.on("data", (d) => { if (sshStderr.length < 4096) sshStderr += d; });

  const health = await waitForTunnel(localHttpPort, 10000);
  if (!health) {
    child.kill();
    return { success: false, error: `tunnel to ${remoteHost} did not come up within 10s (ssh stderr: ${sshStderr.trim().slice(0, 500)})` };
  }
  const remotePeerId = health.peer_id;
  if (!remotePeerId) {
    child.kill();
    return { success: false, error: `tunnel is up but ${remoteHost}'s /health has no peer_id (p2p_enabled=${health.p2p_enabled}) — is P2P enabled on that instance?` };
  }

  const peerAddr = `/ip4/127.0.0.1/tcp/${localP2pPort}/p2p/${remotePeerId}`;
  let connectResult;
  try {
    connectResult = await httpPostJson(`${OZONE_HOST}/network/connect-peer`, { peer_addr: peerAddr }, 10000);
  } catch (e) {
    child.kill();
    return { success: false, error: `tunnel up, but connect-peer call to this host's own API failed: ${e.message}` };
  }

  bridges.set(remoteHost, { child, localHttpPort, localP2pPort, remotePeerId, sshUser, startedAt: Date.now() });
  child.on("exit", () => {
    const b = bridges.get(remoteHost);
    if (b && b.child === child) bridges.delete(remoteHost);
  });

  return {
    success: Boolean(connectResult?.success),
    reused_existing_bridge: false,
    remote_host: remoteHost,
    remote_peer_id: remotePeerId,
    local_http_port: localHttpPort,
    local_p2p_port: localP2pPort,
    peer_addr: peerAddr,
    connect_peer_result: connectResult,
    note: "Tunnel + connect-peer only. This does NOT pair/trust the peer — see POST /network/approve-peer on the real Ozone-Studio API for that, a separate, deliberate decision.",
  };
}

function bridgeDisconnect(input) {
  const remoteHost = String(input.remote_host ?? "");
  const existing = bridges.get(remoteHost);
  if (!existing) {
    return { success: false, error: `no active bridge tracked for '${remoteHost}'` };
  }
  existing.child.kill();
  bridges.delete(remoteHost);
  return {
    success: true,
    remote_host: remoteHost,
    note: "Tunnel torn down. The underlying libp2p connection (if still up) is NOT explicitly disconnected — no disconnect-peer route exists on the real Ozone-Studio API yet (known, flagged gap); it will idle out on its own.",
  };
}

function bridgeStatus() {
  const list = [...bridges.entries()].map(([remoteHost, b]) => ({
    remote_host: remoteHost,
    remote_peer_id: b.remotePeerId,
    local_http_port: b.localHttpPort,
    local_p2p_port: b.localP2pPort,
    alive: b.child.exitCode === null && !b.child.killed,
    uptime_secs: Math.round((Date.now() - b.startedAt) / 1000),
  }));
  return { success: true, bridges: list, allowed_hosts: ALLOWED_HOSTS, locked: ALLOWED_HOSTS.length === 0 };
}

const server = createServer((req, res) => {
  if (req.method === "GET" && (req.url ?? "").startsWith("/status")) {
    reply(res, 200, {
      tool: "p2p-bridge-mcp",
      started_at: new Date(startedAt).toISOString(),
      allowed_hosts: ALLOWED_HOSTS,
      locked: ALLOWED_HOSTS.length === 0,
      active_bridges: bridges.size,
    });
    return;
  }
  if (req.method !== "POST" || !(req.url ?? "").startsWith("/call")) {
    reply(res, 404, { error: "POST /call or GET /status only" });
    return;
  }
  const chunks = [];
  let size = 0;
  req.on("data", (c) => { size += c.length; if (size > 1024 * 1024) req.destroy(); else chunks.push(c); });
  req.on("end", async () => {
    let tool = "";
    let input = {};
    try {
      const body = JSON.parse(Buffer.concat(chunks).toString("utf8") || "{}");
      tool = String(body?.tool ?? "");
      input = body?.input ?? {};
    } catch {
      reply(res, 400, { success: false, error: "invalid JSON body" });
      return;
    }
    try {
      if (tool === "p2p_bridge_connect") {
        reply(res, 200, await bridgeConnect(input));
      } else if (tool === "p2p_bridge_disconnect") {
        reply(res, 200, bridgeDisconnect(input));
      } else if (tool === "p2p_bridge_status") {
        reply(res, 200, bridgeStatus());
      } else {
        reply(res, 200, { success: false, error: `unknown p2p-bridge tool '${tool}' (p2p_bridge_connect | p2p_bridge_disconnect | p2p_bridge_status)` });
      }
    } catch (e) {
      reply(res, 200, { success: false, error: `internal error: ${e.message}` });
    }
  });
});

server.on("error", (err) => {
  if (err.code === "EADDRINUSE") {
    console.error(`[p2p-bridge-mcp] :${PORT} already in use — another instance serves /call; skipping`);
    return;
  }
  console.error("[p2p-bridge-mcp] server error:", err);
});

server.listen(PORT, "127.0.0.1", () => {
  console.error(
    `[p2p-bridge-mcp] /call on 127.0.0.1:${PORT} | allowed hosts: ${ALLOWED_HOSTS.length ? ALLOWED_HOSTS.join(", ") : "LOCKED (no OZONE_P2P_BRIDGE_ALLOWED_HOSTS)"}`
  );
});
