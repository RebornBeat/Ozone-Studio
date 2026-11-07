// Ozone-Studio 3D MCP — 3D-modality (109) asset generation tools.
// 3D assets belong here, under the 3D modality. NOTE (operator correction 2026-11-03): the rat
// model is a standalone 3D ASSET of the workspace — it is NOT part of the EEG tooling (Cerebrix
// is EEG signal windows only). The tool keeps its historical name `eeg_cap_costume_generate`
// for registry compatibility; treat it as "generate the rat 3D asset".
//
// Tools:
//   eeg_cap_costume_generate {output_path?}   (historical name; = the rat 3D asset)
//     → real AnatomicalRatModel.assemble_model() via headless Blender, exported to glTF.
//       HONEST STATUS: the full headless sequence hangs somewhere between body-completion
//       and head-completion — isolated, not root-caused (see eeg_cap_costume_export.py's
//       header). Hard timeout; fails cleanly rather than hanging the MCP.
//   clock_generate {time:'HH:MM', radius_m?, output_dir?}
//     → parametric analog clock via headless Blender → GLB + STL + PNG render.
//   cad_clock_generate {radius_mm?, output_dir?}
//     → solid-model clock via CadQuery/OpenCascade (OZONE_CAD_PYTHON, default ~/.venvs/cad)
//       → STEP + STL. Separate process from Blender — run them one at a time.
//
// Env: OZONE_THREED_PORT (default 3250).

import { createServer } from "node:http";
import { execFile } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const PORT = Number(process.env.OZONE_THREED_PORT ?? 3250);
const CAP_WORKER = path.join(HERE, "eeg_cap_costume_export.py");
const CLOCK_WORKER = path.join(HERE, "clock_blender.py");
const CAD_PY = process.env.OZONE_CAD_PYTHON ?? `${process.env.HOME}/.venvs/cad/bin/python3`;
const CAD_CLOCK = path.join(HERE, "clock_cad.py");

function runBlender(scriptPath, args, timeoutMs = 20000) {
  return new Promise((resolve) => {
    execFile("blender", ["--background", "--python", scriptPath, "--", ...args], { timeout: timeoutMs, maxBuffer: 16 * 1024 * 1024 }, (err, stdout, stderr) => {
      if (err) {
        const timedOut = err.killed || err.signal === "SIGTERM";
        resolve({
          success: false,
          error: timedOut
            ? `blender headless call timed out after ${timeoutMs / 1000}s — known real limitation, see eeg_cap_costume_export.py's header comment`
            : (stderr || err.message || "blender failed").trim(),
        });
        return;
      }
      const jsonLine = stdout.split("\n").reverse().find((l) => l.trim().startsWith("{"));
      if (!jsonLine) { resolve({ success: false, error: `no JSON output from blender worker: ${stdout.slice(-500)}` }); return; }
      try { resolve(JSON.parse(jsonLine)); }
      catch { resolve({ success: false, error: `bad blender worker output: ${jsonLine.slice(0, 300)}` }); }
    });
  });
}

function reply(res, code, body) {
  res.writeHead(code, { "Content-Type": "application/json" });
  res.end(JSON.stringify(body));
}

import { guardOk } from "../shared/guard.mjs";
const server = createServer((req, res) => {
  if (!guardOk(req, res)) return; // loopback guard (tools/shared/guard.mjs)
  if (req.method !== "POST" || !(req.url ?? "").startsWith("/call")) {
    reply(res, 404, { error: "POST /call only" });
    return;
  }
  const chunks = [];
  let size = 0;
  req.on("data", (c) => { size += c.length; if (size > (Number(process.env.OZONE_MCP_MAX_BODY_BYTES) || 512 * 1024 * 1024)) { if (!res.headersSent) { res.writeHead(413, { "Content-Type": "application/json", Connection: "close" }); res.end(JSON.stringify({ success: false, error: `request body is over the transport guard (OZONE_MCP_MAX_BODY_BYTES, default 512 MiB) — NOT processed (nothing was cut); this is a host-memory guard, not a model limit — chunk the call` }), () => req.destroy()); } } else chunks.push(c); });
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
      if (tool === "eeg_cap_costume_generate") {
        const outPath = input.output_path || `/tmp/eeg_cap_costume_${Date.now()}.glb`;
        const out = await runBlender(CAP_WORKER, [outPath], 20000);
        reply(res, 200, out.success ? { success: true, output: out } : out);
      } else if (tool === "clock_generate") {
        // Parametric analog clock via headless Blender -> GLB + STL + PNG render.
        const m = /^(\d{1,2}):(\d{2})$/.exec(String(input.time ?? "10:10"));
        if (!m || Number(m[1]) > 23 || Number(m[2]) > 59) { reply(res, 200, { success: false, error: "time must be HH:MM (00:00-23:59)" }); return; }
        const outDir = input.output_dir || `/tmp/ozone_clock_${Date.now()}`;
        const out = await runBlender(CLOCK_WORKER, [outDir, `${m[1]}:${m[2]}`, String(input.radius_m ?? 0.15)], 120000);
        reply(res, 200, out.success ? { success: true, output: out } : out);
      } else if (tool === "cad_clock_generate") {
        // Solid-model clock (CadQuery/OpenCascade) -> STEP + STL. Separate process from Blender; never run together.
        const outDir = input.output_dir || `/tmp/ozone_cad_clock_${Date.now()}`;
        const out = await new Promise((resolve) => execFile(CAD_PY, [CAD_CLOCK, outDir, String(input.radius_mm ?? 150)], { timeout: 120000, maxBuffer: 16 * 1024 * 1024 }, (err, stdout, stderr) => {
          if (err) { resolve({ success: false, error: (stderr || err.message || "cad failed").trim().slice(-600) }); return; }
          const line = stdout.split("\n").reverse().find((l) => l.trim().startsWith("{"));
          try { resolve(JSON.parse(line)); } catch { resolve({ success: false, error: `bad cad worker output: ${stdout.slice(-300)}` }); }
        }));
        reply(res, 200, out.success ? { success: true, output: out } : out);
      } else {
        reply(res, 200, { success: false, error: `unknown 3d tool '${tool}' (eeg_cap_costume_generate | clock_generate | cad_clock_generate)` });
      }
    } catch (e) {
      reply(res, 200, { success: false, error: e instanceof Error ? e.message : String(e) });
    }
  });
});

server.on("error", (err) => {
  if (err.code === "EADDRINUSE") { console.error(`[threed-mcp] :${PORT} in use — skipping`); return; }
  console.error("[threed-mcp] server error:", err);
});

server.listen(PORT, "127.0.0.1", () => {
  console.error(`[threed-mcp] /call on 127.0.0.1:${PORT} — eeg_cap_costume_generate (known headless-hang limitation)`);
});
