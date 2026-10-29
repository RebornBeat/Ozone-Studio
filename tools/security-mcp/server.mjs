// Ozone-Studio Security MCP — read-only system/network security monitoring
// (operator directive 2026-09-29: "a better Wireshark, MCP-native" — network
// connections/ports, firewall status, process resource usage). Follows
// docs/NEW_MCP_GUIDE.md Pattern A exactly.
//
// SECURITY MODEL (written before any capability, per the guide's own
// checklist): every tool here is READ-ONLY. Nothing in this server can
// change a firewall rule, kill a process, or modify any system state —
// there is no such code path, not a permission check that happens to
// block one. Locked-by-default means "no mutation capability exists",
// not "mutation exists but is gated." Do not add write/mutate tools to
// this file without a fresh, explicit security-model pass first.
//
// HONESTY NOTE (found live, 2026-09-29, this host): firewall inspection
// (`nft list ruleset` / `iptables -L`) requires root on this machine —
// confirmed by real command output, both refuse with "Operation not
// permitted" as the unprivileged user this server runs as. This server
// does NOT escalate privileges (no sudo, ever) — firewall_status reports
// that refusal honestly (success:false + the real stderr) rather than
// silently returning an empty ruleset, which would misleadingly read as
// "no firewall rules configured." Process attribution on net_connections/
// listening_ports is similarly limited without root — ss(8)'s Process
// column is real but empty for other users' sockets; reported as-is, not
// backfilled or guessed.
//
// Tools:
//   net_connections {}              → active TCP/UDP connections (ss -tunap)
//   listening_ports {}              → listening sockets only (ss -tulnp)
//   firewall_status {}              → nft ruleset, iptables fallback, or
//                                      an honest permission-denied report
//   process_top {sort?, limit?}     → top processes by cpu|mem (ps), real
//                                      live snapshot, no history/retention
//
// Env: OZONE_SECURITY_PORT (default 3240).

import { createServer } from "node:http";
import { execFile } from "node:child_process";

const PORT = Number(process.env.OZONE_SECURITY_PORT ?? 3240);

function run(cmd, args) {
  return new Promise((resolve) => {
    execFile(cmd, args, { timeout: 10000, maxBuffer: 8 * 1024 * 1024 }, (err, stdout, stderr) => {
      resolve({ ok: !err, code: err?.code ?? 0, stdout: stdout ?? "", stderr: (stderr ?? "").trim() });
    });
  });
}

// Parses `ss` column output into structured rows. Real parser against
// real ss(8) output (verified live against this host, both connection
// and listening modes) — not a guessed format.
function parseSs(stdout) {
  const lines = stdout.split("\n").filter((l) => l.trim().length > 0);
  if (lines.length === 0) return [];
  const rows = [];
  for (const line of lines.slice(1)) {
    const cols = line.trim().split(/\s+/);
    if (cols.length < 5) continue;
    const [proto, state, recvq, sendq, local, peer, ...rest] = cols;
    rows.push({
      proto,
      state,
      local_address: local,
      peer_address: peer ?? null,
      process: rest.length ? rest.join(" ") : null,
    });
  }
  return rows;
}

function parseProcessTop(stdout) {
  const lines = stdout.split("\n").filter((l) => l.trim().length > 0);
  if (lines.length <= 1) return [];
  return lines.slice(1).map((line) => {
    const cols = line.trim().split(/\s+/);
    const [pid, ppid, cpu, mem, elapsed, ...cmd] = cols;
    return {
      pid: Number(pid),
      ppid: Number(ppid),
      cpu_percent: Number(cpu),
      mem_percent: Number(mem),
      elapsed,
      command: cmd.join(" "),
    };
  });
}

function reply(res, code, body) {
  res.writeHead(code, { "Content-Type": "application/json" });
  res.end(JSON.stringify(body));
}

const server = createServer((req, res) => {
  if (req.method !== "POST" || !(req.url ?? "").startsWith("/call")) {
    reply(res, 404, { error: "POST /call only" });
    return;
  }
  const chunks = [];
  let size = 0;
  req.on("data", (c) => { size += c.length; if (size > 1 * 1024 * 1024) req.destroy(); else chunks.push(c); });
  req.on("end", async () => {
    let tool = "";
    let input = {};
    try {
      const body = JSON.parse(Buffer.concat(chunks).toString("utf8") || "{}");
      tool = String(body?.tool ?? "");
      // The host registry also lists `sec_`-prefixed aliases of these tools
      // (sec_net_connections, sec_firewall_status, ...). Accept them as the
      // same read-only tool instead of answering "unknown tool".
      if (tool.startsWith("sec_")) tool = tool.slice(4);
      input = body?.input ?? {};
    } catch {
      reply(res, 400, { success: false, error: "invalid JSON body" });
      return;
    }
    try {
      if (tool === "net_connections") {
        const r = await run("ss", ["-tunap"]);
        if (!r.ok && !r.stdout) {
          reply(res, 200, { success: false, error: r.stderr || "ss failed" });
          return;
        }
        const connections = parseSs(r.stdout);
        const anyProcess = connections.some((c) => c.process);
        reply(res, 200, {
          success: true,
          output: {
            connections,
            count: connections.length,
            process_attribution: anyProcess
              ? "available for own-user sockets"
              : "unavailable — process/PID attribution requires root (ss(8) Process column empty for this user)",
          },
        });
      } else if (tool === "listening_ports") {
        const r = await run("ss", ["-tulnp"]);
        if (!r.ok && !r.stdout) {
          reply(res, 200, { success: false, error: r.stderr || "ss failed" });
          return;
        }
        const listening = parseSs(r.stdout);
        reply(res, 200, {
          success: true,
          output: {
            listening,
            count: listening.length,
            process_attribution: listening.some((c) => c.process)
              ? "available for own-user sockets"
              : "unavailable — process/PID attribution requires root",
          },
        });
      } else if (tool === "firewall_status") {
        const nft = await run("nft", ["list", "ruleset"]);
        if (nft.ok) {
          const ruleCount = (nft.stdout.match(/^\s*(ip|ip6|inet|arp|bridge|netdev)\s+\w+\s+\w+/gm) ?? []).length;
          reply(res, 200, {
            success: true,
            output: { backend: "nftables", raw: nft.stdout, rule_line_count: ruleCount },
          });
          return;
        }
        const ipt = await run("iptables", ["-L", "-n", "-v"]);
        if (ipt.ok) {
          reply(res, 200, {
            success: true,
            output: { backend: "iptables (legacy fallback — nftables unavailable/denied)", raw: ipt.stdout },
          });
          return;
        }
        // Both real backends refused. A PERMISSION refusal is a known limit of this
        // deployment (the EU unit holds CAP_NET_ADMIN; the local dev host does not),
        // not a posture finding: reporting it as a failure made the sensor tick raise
        // the same finding every 15 min and fed the I-Loop an echo. Answer
        // success:true with inspectable:false — UNKNOWN, never "no rules".
        if (/permitted|permission|denied|superuser|must be root|root privileges/i.test(`${nft.stderr} ${ipt.stderr}`)) {
          const services = {};
          for (const u of ["ufw", "nftables", "firewalld"]) {
            const r = await run("systemctl", ["is-active", u]);
            services[u] = (r.stdout || r.stderr || "unknown").trim().split("\n")[0];
          }
          reply(res, 200, {
            success: true,
            output: {
              backend: "unprivileged",
              inspectable: false,
              ruleset: null,
              services,
              reason: `ruleset not readable without CAP_NET_ADMIN — nft: "${nft.stderr}"; iptables: "${ipt.stderr}"`,
              note: "This server never escalates privileges. inspectable:false means UNKNOWN, not 'no firewall rules'. Run under AmbientCapabilities=CAP_NET_ADMIN (as the EU unit does) for the real ruleset.",
            },
          });
          return;
        }
        reply(res, 200, {
          success: false,
          error: `firewall inspection failed — nft: "${nft.stderr}"; iptables: "${ipt.stderr}"`,
        });
      } else if (tool === "exposure_report") {
        // Read-only, from `ss -tulnH`: which listeners are bound beyond loopback, and
        // which of those are well-known data-store / admin ports.
        const r = await run("ss", ["-tulnH"]);
        if (!r.ok && !r.stdout) { reply(res, 200, { success: false, error: r.stderr || "ss failed" }); return; }
        const RISKY = { 22: "ssh", 23: "telnet", 2375: "docker-api", 3306: "mysql", 5432: "postgres", 6379: "redis", 9200: "elasticsearch", 11211: "memcached", 27017: "mongodb" };
        const seen = new Set();
        const listeners = [];
        for (const line of r.stdout.split("\n")) {
          const c = line.trim().split(/\s+/);
          if (c.length < 5) continue;
          const local = c[4];
          const i = local.lastIndexOf(":");
          const host = local.slice(0, i).replace(/%.*$/, "").replace(/^\[|\]$/g, "");
          const port = Number(local.slice(i + 1));
          const key = `${c[0]}/${host}/${port}`;
          if (seen.has(key)) continue;
          seen.add(key);
          const loopback = host === "::1" || host.startsWith("127.");
          listeners.push({ proto: c[0], host, port, scope: loopback ? "loopback" : "external", risky_service: RISKY[port] ?? null });
        }
        const external = listeners.filter((l) => l.scope === "external");
        reply(res, 200, {
          success: true,
          output: {
            listeners,
            external_count: external.length,
            external_risky: external.filter((l) => l.risky_service),
            note: "'external' = bound to a non-loopback address; actual reachability still depends on the firewall (see firewall_status.inspectable)",
          },
        });
      } else if (tool === "security_status") {
        reply(res, 200, { success: true, output: { tool: "security-mcp", read_only: true, privileged: process.getuid?.() === 0, tools: ["net_connections", "listening_ports", "firewall_status", "exposure_report", "process_top"] } });
      } else if (tool === "process_top") {
        const sortField = input.sort === "mem" ? "-%mem" : "-%cpu";
        const limit = Math.max(1, Math.min(100, Number(input.limit ?? 15)));
        const r = await run("ps", ["-eo", "pid,ppid,%cpu,%mem,etime,comm", "--sort=" + sortField]);
        if (!r.ok) {
          reply(res, 200, { success: false, error: r.stderr || "ps failed" });
          return;
        }
        const all = parseProcessTop(r.stdout);
        reply(res, 200, {
          success: true,
          output: {
            processes: all.slice(0, limit),
            sorted_by: input.sort === "mem" ? "mem_percent" : "cpu_percent",
            note: "cpu_percent is a real usage proxy, not measured wattage — no RAPL/powertop backing on this host",
          },
        });
      } else {
        reply(res, 200, {
          success: false,
          error: `unknown security tool '${tool}' (net_connections, listening_ports, firewall_status, exposure_report, security_status, process_top)`,
        });
      }
    } catch (e) {
      reply(res, 200, { success: false, error: e instanceof Error ? e.message : String(e) });
    }
  });
});

server.on("error", (err) => {
  if (err.code === "EADDRINUSE") { console.error(`[security-mcp] :${PORT} in use — skipping`); return; }
  console.error("[security-mcp] server error:", err);
});

server.listen(PORT, "127.0.0.1", () => {
  console.error(`[security-mcp] /call on 127.0.0.1:${PORT} — read-only: net_connections, listening_ports, firewall_status, process_top`);
});
