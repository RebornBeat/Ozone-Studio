// Ozone-Studio 3D MCP — 3D-modality (109) asset generation tools.
// Split out of tools/cerebrix-mcp/ (operator correction, 2026-09-29):
// Cerebrix is EEG-only; 3D/Blender asset generation belongs here, under
// the 3D modality, not under an EEG-branded MCP. First tool wraps the
// legacy rat-model generator, whose real purpose is EEG-CAP COSTUME
// SIMULATION — an anatomically-measured 3D rat used to design/fit a
// physical EEG electrode cap, not a general anatomical asset.
//
// Tools:
//   eeg_cap_costume_generate {output_path?}
//     → real AnatomicalRatModel.assemble_model() via headless Blender,
//       exported to glTF. HONEST STATUS (carried over from the original
//       build+diagnosis): the full headless sequence hangs somewhere
//       between body-completion and head-completion — isolated but not
//       root-caused (see eeg_cap_costume_export.py's header). Wrapped
//       with a hard timeout; fails cleanly rather than hanging the MCP.
//
// Env: OZONE_THREED_PORT (default 3250).

import { createServer } from "node:http";
import { execFile } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const PORT = Number(process.env.OZONE_THREED_PORT ?? 3250);
const CAP_WORKER = path.join(HERE, "eeg_cap_costume_export.py");

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
      if (tool === "eeg_cap_costume_generate") {
        const outPath = input.output_path || `/tmp/eeg_cap_costume_${Date.now()}.glb`;
        const out = await runBlender(CAP_WORKER, [outPath], 20000);
        reply(res, 200, out.success ? { success: true, output: out } : out);
      } else {
        reply(res, 200, { success: false, error: `unknown 3d tool '${tool}' (eeg_cap_costume_generate)` });
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
