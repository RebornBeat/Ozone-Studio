// Ozone-Studio Code MCP — wraps the code modality pipeline (101) with a
// call-graph tool (operator directive 2026-09-29): entry-point detection +
// call ordering for code workspaces/projects. The pipeline owns the graph
// (Calls edges, ordering attributes); this MCP is the /mcp/call surface —
// metered, gated, rippled, reviewed.
//
// Tools:
//   code_callgraph {input: {code, language?, file_path?, persist?, project_id?}}
//     → {entry_points, call_sequence, functions[{name, call_order, is_entry_point}]}
//     persist:true (default) also creates the ModalityGraph container.
//
// Env: OZONE_CODE_PORT (default 3230),
//      OZONE_CODE_PIPELINE (default .../modalities/code/target/release/code),
//      OZONE_HOST, OZONE_ZSEI_DATA_DIR (passed through to the pipeline).

import { createServer } from "node:http";
import { spawn } from "node:child_process";

const PORT = Number(process.env.OZONE_CODE_PORT ?? 3230);
const PIPELINE = process.env.OZONE_CODE_PIPELINE
  ?? "/home/rebornbeat/Projects/Ozone-Studio/assets/pipelines/modalities/code/target/release/code";
// Real bug found 2026-09-29 (verifying container 40425): passing an empty
// string here (`process.env.OZONE_ZSEI_DATA_DIR ?? ""`) is NOT the same as
// leaving the var unset — the pipeline's own `env::var(...).unwrap_or_else
// (|_| "zsei_data".to_string())` only falls back on a missing var, so an
// explicit empty string became data_dir="" and content wrote to a broken
// `/graphs/...` path (root-level, doesn't exist) while the container's
// object_store_path metadata still claimed success. Matches visual-mcp's
// already-correct real-absolute-path default.
const ZSEI_DATA_DIR = process.env.OZONE_ZSEI_DATA_DIR
  ?? "/home/rebornbeat/Projects/Ozone-Studio/target/release/zsei_data";

function runPipeline(inputObj, extraEnv = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(PIPELINE, ["--input", JSON.stringify(inputObj)], {
      timeout: 60000,
      env: { ...process.env, ...extraEnv },
    });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (d) => { stdout += d; });
    child.stderr.on("data", (d) => { stderr += d; });
    child.on("error", (err) => reject(new Error(`code pipeline spawn failed: ${err.message}`)));
    child.on("close", (code) => {
      try { resolve(JSON.parse(stdout)); }
      catch { reject(new Error(`code pipeline bad output (exit ${code}): ${(stderr || stdout).slice(0, 200)}`)); }
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
      if (tool === "code_callgraph") {
        const code = String(input.code ?? "");
        if (!code) { reply(res, 200, { success: false, error: "code required" }); return; }
        const analyze = {
          data: { action: { type: "Analyze", code, language: input.language ?? null, file_path: input.file_path ?? null } },
          context: {},
        };
        const out = await runPipeline(analyze);
        if (!out.success) throw new Error(out.error ?? "Analyze failed");
        const analysis = out.analysis ?? {};
        const result = {
          entry_points: analysis.entry_points ?? [],
          call_sequence: analysis.call_sequence ?? [],
          functions: (analysis.functions ?? []).map((f) => ({
            name: f.name,
            call_order: f.call_order ?? null,
            is_entry_point: f.is_entry_point ?? false,
            calls: f.calls ?? [],
            start_line: f.start_line,
          })),
        };
        if (input.persist) {
          const created = await runPipeline({
            data: { action: { type: "CreateGraph", analysis_result: analysis, project_id: Number(input.project_id ?? 3), link_to_existing: false } },
            context: {},
          }, { OZONE_HOST: process.env.OZONE_HOST ?? "http://127.0.0.1:50051", OZONE_ZSEI_DATA_DIR: ZSEI_DATA_DIR });
          result.graph_id = created.graph_id ?? (created.result ?? {}).CreateGraph?.graph_id ?? (created.result ?? {}).graph_id ?? null;
          result.persisted = created.success;
        }
        reply(res, 200, { success: true, output: result });
      } else {
        reply(res, 200, { success: false, error: `unknown code tool '${tool}' (code_callgraph)` });
      }
    } catch (e) {
      reply(res, 200, { success: false, error: e instanceof Error ? e.message : String(e) });
    }
  });
});

server.on("error", (err) => {
  if (err.code === "EADDRINUSE") { console.error(`[code-mcp] :${PORT} in use — skipping`); return; }
  console.error("[code-mcp] server error:", err);
});

server.listen(PORT, "127.0.0.1", () => {
  console.error(`[code-mcp] /call on 127.0.0.1:${PORT} | pipeline: ${PIPELINE}`);
});
