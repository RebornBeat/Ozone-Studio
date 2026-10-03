// Ozone-Studio Web-Search MCP — wraps the BUILT web_search pipeline (56)
// (docs/TOOLS_PIPELINES_MCP_GUIDE.md §4 contract instance; capability-gap
// scan item #1). Real Brave Search via the pipeline's own provider auth —
// the MCP adds the /mcp/call surface: metered, gated, rippled, reviewed.
//
// Tools:
//   web_search {input: {query, max_results?}} → real Brave results
//   current_datetime {}                        → the pipeline's real clock action
//
// Env: OZONE_WEB_PORT (default 3225),
//      OZONE_WEB_PIPELINE (default .../general/web_search/target/release/web_search)

import { createServer } from "node:http";
import { spawn } from "node:child_process";

const PORT = Number(process.env.OZONE_WEB_PORT ?? 3225);
const PIPELINE = process.env.OZONE_WEB_PIPELINE
  ?? "/home/rebornbeat/Projects/Ozone-Studio/assets/pipelines/general/web_search/target/release/web_search";

function runPipeline(inputObj) {
  // web_search's parse_cli_input reads --input ARG ONLY (no stdin fallback)
  const json = JSON.stringify(inputObj);
  return new Promise((resolve, reject) => {
    const child = spawn(PIPELINE, ["--input", json], { timeout: 60000 });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (d) => { stdout += d; });
    child.stderr.on("data", (d) => { stderr += d; });
    child.on("error", (err) => reject(new Error(`web_search spawn failed: ${err.message}`)));
    child.on("close", (code) => {
      try { resolve(JSON.parse(stdout)); }
      catch { reject(new Error(`web_search bad output (exit ${code}): ${(stderr || stdout).slice(0, 200)}`)); }
    });
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
      if (tool === "web_search") {
        const query = String(input.query ?? "");
        if (!query) { reply(res, 200, { success: false, error: "query required" }); return; }
        const out = await runPipeline({
          action: { type: "Search", query, max_results: Number(input.max_results ?? 5) },
        });
        reply(res, 200, { success: out.success !== false, output: { results: out.results ?? [], error: out.error } });
      } else if (tool === "current_datetime") {
        const out = await runPipeline({ action: { type: "CurrentDateTime" } });
        reply(res, 200, { success: out.success !== false, output: out.result ?? out });
      } else {
        reply(res, 200, { success: false, error: `unknown web tool '${tool}' (web_search | current_datetime)` });
      }
    } catch (e) {
      reply(res, 200, { success: false, error: e instanceof Error ? e.message : String(e) });
    }
  });
});

server.on("error", (err) => {
  if (err.code === "EADDRINUSE") { console.error(`[web-mcp] :${PORT} in use — skipping`); return; }
  console.error("[web-mcp] server error:", err);
});

server.listen(PORT, "127.0.0.1", () => {
  console.error(`[web-mcp] /call on 127.0.0.1:${PORT} | pipeline: ${PIPELINE}`);
});
