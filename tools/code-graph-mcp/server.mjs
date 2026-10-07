// Ozone-Studio Code Graph MCP — static call graph for Python sources.
//
// Tool: code_call_graph {input: {path}}
//   path: a .py file or directory inside the configured root (default: the repository root).
//   Output: a graph block (CodeRoot → Module → Class/Function) with ImportsFrom and
//   CallsTo edges, plus the unresolved imports and calls that could not be attributed.
//
// Resolution is conservative: an edge is emitted only when the callee is unique inside the
// scanned set. Dynamic, ambiguous and external calls are reported, never guessed.
//
// Edge relation names: the ZSEI relation set has no call or import types, so CallsTo maps
// to "CallsTo" and ImportsFrom to "ImportsFrom" (ZSEI RelationType variants). The kind is also kept on each edge
// as `kind` in the output; the host persists only the mapped relation name.
//
// Env: OZONE_CODE_GRAPH_PORT (default 3279), OZONE_CODE_GRAPH_ROOT (default repository root),
//      OZONE_CODE_GRAPH_PYTHON (default python3; stdlib only, no venv needed).

import { createServer } from "node:http";
import { execFile } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const PORT = Number(process.env.OZONE_CODE_GRAPH_PORT ?? 3279);
const ROOT = path.resolve(process.env.OZONE_CODE_GRAPH_ROOT ?? path.join(HERE, "..", ".."));
const PYTHON = process.env.OZONE_CODE_GRAPH_PYTHON ?? "python3";
const WORKER = path.join(HERE, "code_worker.py");

function reply(res, code, body) {
  res.writeHead(code, { "Content-Type": "application/json" });
  res.end(JSON.stringify(body));
}

function runWorker(relPath) {
  return new Promise((resolve, reject) => {
    const payload = JSON.stringify({ root: ROOT, path: relPath });
    execFile(PYTHON, [WORKER, payload], { timeout: 120000, maxBuffer: 64 * 1024 * 1024 }, (err, stdout, stderr) => {
      if (err) {
        reject(new Error(`code graph worker failed: ${String(stderr || err.message).trim().slice(0, 400)}`));
        return;
      }
      const line = stdout.trim().split("\n").pop() ?? "";
      try {
        resolve(JSON.parse(line));
      } catch {
        reject(new Error(`code graph worker produced no JSON: ${stdout.slice(0, 200)}`));
      }
    });
  });
}

const server = createServer((req, res) => {
  if (req.method !== "POST" || !(req.url ?? "").startsWith("/call")) {
    reply(res, 404, { error: "POST /call only" });
    return;
  }
  const chunks = [];
  req.on("data", (c) => chunks.push(c));
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
    if (tool !== "code_call_graph") {
      reply(res, 400, { success: false, error: `unknown tool '${tool}' (code_call_graph)` });
      return;
    }
    const rel = String(input.path ?? "").trim();
    if (!rel) {
      reply(res, 400, { success: false, error: "path is required (a .py file or directory inside the root)" });
      return;
    }
    if (path.isAbsolute(rel)) {
      reply(res, 400, { success: false, error: "path must be relative to the configured root" });
      return;
    }
    try {
      const out = await runWorker(rel);
      if (!out.ok) {
        reply(res, 200, { success: false, error: out.error });
        return;
      }
      reply(res, 200, { success: true, output: { path: rel, root: ROOT, ...out } });
    } catch (e) {
      reply(res, 200, { success: false, error: e instanceof Error ? e.message : String(e) });
    }
  });
});

server.on("error", (err) => {
  if (err.code === "EADDRINUSE") {
    console.error(`[code-graph-mcp] :${PORT} in use — skipping`);
    return;
  }
  console.error(`[code-graph-mcp] server error: ${err.message}`);
});

server.listen(PORT, "127.0.0.1", () => {
  console.error(`[code-graph-mcp] /call on 127.0.0.1:${PORT} — code_call_graph, root ${ROOT}`);
});
