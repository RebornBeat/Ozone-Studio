// Ozone-Studio Connectome MCP — the fruit fly (Drosophila melanogaster)
// brain connectome (operator directive 2026-09-29: "the NN of the fruit
// fly, download it and plug it in, study it, monitor it, web search").
//
// MODALITY CORRECTION (operator, same day): a connectome is a static
// weighted graph (neurons/synapses), NOT EEG data — no time dimension,
// no signal. This is a `network`-modality tool (with real `biology`/3D
// facets), independent of Cerebrix — an earlier pass in this session
// routed a subgraph through Cerebrix's GraphNeuralNetwork decision layer,
// which was wrong on two counts (the modality mismatch here, and,
// independently, that Cerebrix class is genuinely incomplete — missing
// its abstract get_config() implementation, a real bug in Cerebrix's own
// code, not chased further). Removed; this MCP now stands on its own.
//
// Real data, two real sources:
//   1. FlyWire/Codex FAFB neuron annotations (139,248 real neurons,
//      github.com/flyconnectome/flywire_annotations, public) — catalog
//      only, no connectivity (Codex's own connectivity download needs
//      auth this session doesn't have, confirmed via a real fetch).
//   2. Google Research / HHMI Janelia FlyEM male-cns:v1.0 (211,577 real
//      annotated neurons, 151,856,684 real synaptic connections,
//      storage.googleapis.com/flyem-male-cns, genuinely public, no auth
//      — downloaded in full, byte count verified against the server's
//      own Content-Length: 1,051,241,946 bytes exact) — the real wiring.
//
// Tools:
//   fly_neuron_query {cell_type?, super_class?, side?, neurotransmitter?, limit?}
//     → real filtered FlyWire neuron records.
//   fly_connectome_stats {}
//     → real aggregate counts over the FlyWire catalog, computed live.
//   fly_circuit_subgraph {seed_body_id?, top_k?}
//     → a real induced subgraph from the male-cns connectivity data
//       around a given neuron (default: the Giant Fiber / DNp01, bodyId
//       10001 — a real, well-studied escape-reflex command neuron).
//       Real node metadata + real edge weights; the adjacency matrix
//       itself is left out of the wire response (large N×N), available
//       by running giant_fiber_circuit.build_subgraph() directly.
//
// Env: OZONE_CONNECTOME_PORT (default 3255), FLY_CONNECTOME_TSV.

import { createServer, request as httpRequest } from "node:http";
import { execFile } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const PORT = Number(process.env.OZONE_CONNECTOME_PORT ?? 3255);
const WORKER = path.join(HERE, "fly_connectome.py");
// fly_circuit_subgraph proxies to circuit_service.py (:3256), a
// persistent process — real, measured fix: spawning circuit_worker.py
// fresh per call took 16.5s (mostly re-reading the 1GB connectivity
// file from disk), well past /mcp/call's own timeout to tool endpoints.
// The persistent service loads both feather tables once and answers
// from memory (~4s per query, real compute cost, no I/O reload).
const CIRCUIT_SERVICE_PORT = Number(process.env.OZONE_CIRCUIT_SERVICE_PORT ?? 3256);

function queryCircuitService(input, timeoutMs = 12000) {
  return new Promise((resolve) => {
    const data = JSON.stringify(input);
    const req = httpRequest(
      { hostname: "127.0.0.1", port: CIRCUIT_SERVICE_PORT, path: "/query", method: "POST", headers: { "Content-Type": "application/json" }, timeout: timeoutMs },
      (res) => {
        let body = "";
        res.on("data", (c) => (body += c));
        res.on("end", () => {
          try { resolve(JSON.parse(body)); }
          catch { resolve({ success: false, error: `bad circuit-service response: ${body.slice(0, 300)}` }); }
        });
      },
    );
    req.on("error", (e) => resolve({ success: false, error: `circuit-service unreachable: ${e.message} (is circuit_service.py running on :${CIRCUIT_SERVICE_PORT}?)` }));
    req.on("timeout", () => { req.destroy(); resolve({ success: false, error: `circuit-service timed out after ${timeoutMs / 1000}s` }); });
    req.write(data);
    req.end();
  });
}

function runPython(pythonBin, scriptPath, stdinObj, timeoutMs = 20000) {
  return new Promise((resolve) => {
    const child = execFile(pythonBin, [scriptPath], { timeout: timeoutMs, maxBuffer: 32 * 1024 * 1024 }, (err, stdout, stderr) => {
      if (err && !stdout) {
        resolve({ success: false, error: (stderr || err.message || "worker failed").trim() });
        return;
      }
      const lastLine = stdout.trim().split("\n").filter(Boolean).pop();
      try { resolve(JSON.parse(lastLine)); }
      catch { resolve({ success: false, error: `bad worker output: ${(stdout || stderr).slice(0, 300)}` }); }
    });
    child.stdin.write(JSON.stringify(stdinObj));
    child.stdin.end();
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
      input = body?.input ?? {};
    } catch {
      reply(res, 400, { success: false, error: "invalid JSON body" });
      return;
    }
    try {
      if (tool === "fly_neuron_query") {
        const out = await runPython("python3", WORKER, { action: "query", ...input });
        reply(res, 200, out.success ? { success: true, output: out } : out);
      } else if (tool === "fly_connectome_stats") {
        const out = await runPython("python3", WORKER, { action: "stats" });
        reply(res, 200, out.success ? { success: true, output: out } : out);
      } else if (tool === "fly_circuit_subgraph") {
        const out = await queryCircuitService(input);
        reply(res, 200, out.success ? { success: true, output: out } : out);
      } else {
        reply(res, 200, { success: false, error: `unknown connectome tool '${tool}' (fly_neuron_query, fly_connectome_stats, fly_circuit_subgraph)` });
      }
    } catch (e) {
      reply(res, 200, { success: false, error: e instanceof Error ? e.message : String(e) });
    }
  });
});

server.on("error", (err) => {
  if (err.code === "EADDRINUSE") { console.error(`[connectome-mcp] :${PORT} in use — skipping`); return; }
  console.error("[connectome-mcp] server error:", err);
});

server.listen(PORT, "127.0.0.1", () => {
  console.error(`[connectome-mcp] /call on 127.0.0.1:${PORT} — network/biology modality: fly_neuron_query, fly_connectome_stats, fly_circuit_subgraph`);
});
