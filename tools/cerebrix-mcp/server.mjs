// Ozone-Studio Cerebrix MCP — the ONE EEG-modality MCP (operator,
// 2026-09-29: "why two different MCPs when they are both on EEG data" —
// correct call; this server holds every EEG data source, not one per
// dataset). Wraps the real Cerebrix BCI AI Controller API
// (~/Projects/Cerebrix) plus two real, downloaded validation/research
// datasets. 3D-model generation (the EEG-cap costume tool) stays in
// tools/threed-mcp/ — that's a 3D-modality concern, not EEG signal data,
// per the same operator correction.
//
// Tools:
//   eeg_window {data | sample:{label,index}, window_size?, stride?, context_size?}
//     → real WindowProcessor.create_sliding_windows() (Cerebrix's own,
//       unmodified). `data` accepts caller-supplied (n_frames,
//       n_channels, n_frequencies) arrays; `sample` pulls a real
//       recording from the downloaded Sentdex/BCI dataset (1,230 real
//       left/right/none 16-channel sessions, recovered via the Wayback
//       Machine — see eeg_window.py's header for full provenance).
//   karaone_trial {index | prompt, subject?}
//     → one real KaraOne imagined-speech trial's prompt label + real
//       EEG feature matrix shape/summary — the real "1-word command
//       vocabulary per person" dataset (4 real words: pat/pot/knew/gnaw,
//       + 7 phonemes, 15 real trials each, subject MM05 downloaded and
//       verified in full). See karaone.py's header for the honest
//       trial-alignment caveat (operator-flagged, not glossed over).
//   karaone_stats {subject?}
//     → real per-subject label counts, computed live.
//
// Not yet wired (honest scope boundary): Cerebrix's full ML pipeline
// (feature extraction → CNN-LSTM/GNN/HAN/Transformer → intent) needs
// tensorflow/pylsl beyond what's installed for these two data tools.
//
// Env: OZONE_CEREBRIX_PORT (default 3245), CEREBRIX_ROOT.

import { createServer } from "node:http";
import { execFile } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const PORT = Number(process.env.OZONE_CEREBRIX_PORT ?? 3245);
const EEG_WORKER = path.join(HERE, "eeg_window.py");
const KARAONE_WORKER = path.join(HERE, "karaone.py");
const VENV_PYTHON = path.join(HERE, ".venv", "bin", "python3");

function runPython(pythonBin, scriptPath, stdinObj, timeoutMs = 15000) {
  return new Promise((resolve) => {
    const child = execFile(pythonBin, [scriptPath], { timeout: timeoutMs, maxBuffer: 16 * 1024 * 1024 }, (err, stdout, stderr) => {
      if (err && !stdout) {
        resolve({ success: false, error: (stderr || err.message || "python worker failed").trim() });
        return;
      }
      try { resolve(JSON.parse(stdout.trim().split("\n").pop())); }
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
  req.on("data", (c) => { size += c.length; if (size > 20 * 1024 * 1024) req.destroy(); else chunks.push(c); });
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
      if (tool === "eeg_window") {
        if (!Array.isArray(input.data) && !input.sample) {
          reply(res, 200, { success: false, error: "provide either data: (n_frames, n_channels, n_frequencies) nested array, or sample: {label, index} for a real Sentdex recording" });
          return;
        }
        const out = await runPython("python3", EEG_WORKER, input);
        reply(res, 200, out.success ? { success: true, output: out } : out);
      } else if (tool === "karaone_trial") {
        const out = await runPython(VENV_PYTHON, KARAONE_WORKER, { action: "trial", ...input });
        reply(res, 200, out.success ? { success: true, output: out } : out);
      } else if (tool === "karaone_stats") {
        const out = await runPython(VENV_PYTHON, KARAONE_WORKER, { action: "stats", ...input });
        reply(res, 200, out.success ? { success: true, output: out } : out);
      } else {
        reply(res, 200, { success: false, error: `unknown cerebrix tool '${tool}' (eeg_window, karaone_trial, karaone_stats)` });
      }
    } catch (e) {
      reply(res, 200, { success: false, error: e instanceof Error ? e.message : String(e) });
    }
  });
});

server.on("error", (err) => {
  if (err.code === "EADDRINUSE") { console.error(`[cerebrix-mcp] :${PORT} in use — skipping`); return; }
  console.error("[cerebrix-mcp] server error:", err);
});

server.listen(PORT, "127.0.0.1", () => {
  console.error(`[cerebrix-mcp] /call on 127.0.0.1:${PORT} — EEG only: eeg_window, karaone_trial, karaone_stats`);
});
