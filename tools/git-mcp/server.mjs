// Ozone-Studio Git MCP — native file-history tools over the file beacon's
// captures (operator vision 2026-09-29: a git-like system NATIVE to the
// graph — per-file, per-workspace/project, all changes, light). Not git
// the binary: history lives in zsei_data/file_history/ (JSONL per
// container + capped snapshots), written by src/file_beacon.rs on every
// real change it detects.
//
// Tools:
//   file_history {input: {container_id}}          → all recorded changes
//   file_status  {}                                → watched files w/ change counts
//   file_diff    {input: {container_id, from_ts?, to_ts?}} → unified diff between snapshots
//
// Env: OZONE_GIT_PORT (default 3235), OZONE_ZSEI_DATA_DIR.

import { createServer } from "node:http";
import fs from "node:fs";
import path from "node:path";

const PORT = Number(process.env.OZONE_GIT_PORT ?? 3235);
const DATA_DIR = process.env.OZONE_ZSEI_DATA_DIR
  ?? "/home/rebornbeat/Projects/Ozone-Studio/target/release/zsei_data";
const HIST_DIR = path.join(DATA_DIR, "file_history");

function readHistory(containerId) {
  const p = path.join(HIST_DIR, `${containerId}.jsonl`);
  if (!fs.existsSync(p)) return [];
  return fs.readFileSync(p, "utf8").split("\n").filter(Boolean)
    .map((l) => { try { return JSON.parse(l); } catch { return null; } })
    .filter(Boolean)
    .sort((a, b) => a.ts - b.ts);
}

function latestSnapshot(containerId, beforeTs = Infinity) {
  const dir = path.join(HIST_DIR, "snapshots", String(containerId));
  if (!fs.existsSync(dir)) return null;
  const snaps = fs.readdirSync(dir)
    .filter((f) => f.endsWith(".snap"))
    .map((f) => {
      const base = f.replace(".snap", "");
      const millis = Number(base.split("_")[0]);
      return { file: path.join(dir, f), millis, size: Number(base.split("_")[1] ?? 0) };
    })
    .filter((s) => s.millis <= beforeTs)
    .sort((a, b) => a.millis - b.millis);
  return snaps.at(-1) ?? null;
}

function diffText(a, b) {
  const al = a.split("\n"); const bl = b.split("\n");
  const out = [];
  let i = 0; let j = 0;
  while (i < al.length || j < bl.length) {
    if (al[i] === bl[j]) { i++; j++; continue; }
    // simple forward-scan diff (good enough for change summaries)
    const del = [];
    while (i < al.length && al[i] !== bl[j]) { del.push(`- ${al[i]}`); i++; }
    const add = [];
    while (j < bl.length && (i >= al.length || al[i] !== bl[j])) { add.push(`+ ${bl[j]}`); j++; }
    out.push(...del, ...add);
    if (i < al.length && j < bl.length) { i++; j++; }
  }
  return out.slice(0, 500);
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
  req.on("data", (c) => chunks.push(c));
  req.on("end", () => {
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
      if (tool === "file_history") {
        const cid = String(input.container_id ?? "");
        const hist = readHistory(cid);
        reply(res, 200, { success: true, output: { container_id: cid, changes: hist.length, history: hist } });
      } else if (tool === "file_status") {
        // Every watched file: one row per history file
        const out = [];
        if (fs.existsSync(HIST_DIR)) {
          for (const f of fs.readdirSync(HIST_DIR).filter((x) => x.endsWith(".jsonl"))) {
            const cid = f.replace(".jsonl", "");
            const hist = readHistory(cid);
            if (hist.length) {
              out.push({
                container_id: cid,
                path: hist.at(-1).path,
                changes: hist.length,
                last_change_ts: hist.at(-1).ts,
                size: hist.at(-1).size,
              });
            }
          }
        }
        reply(res, 200, { success: true, output: { watched: out.sort((a, b) => b.last_change_ts - a.last_change_ts) } });
      } else if (tool === "file_diff") {
        const cid = String(input.container_id ?? "");
        const hist = readHistory(cid);
        const fromTs = Number(input.from_ts ?? 0);
        const toTs = Number(input.to_ts ?? Date.now() + 1e9);
        const fromSnap = latestSnapshot(cid, Math.max(fromTs, 0));
        const toSnap = latestSnapshot(cid, toTs);
        if (!fromSnap || !toSnap || fromSnap.file === toSnap.file) {
          reply(res, 200, { success: true, output: { diff: "", note: "no distinct snapshot pair in range" } });
          return;
        }
        const a = fs.readFileSync(fromSnap.file, "utf8");
        const b = fs.readFileSync(toSnap.file, "utf8");
        reply(res, 200, {
          success: true,
          output: {
            from: { ts: fromSnap.millis, file: fromSnap.file },
            to: { ts: toSnap.millis, file: toSnap.file },
            diff: diffText(a, b),
          },
        });
      } else {
        reply(res, 200, { success: false, error: `unknown git tool '${tool}' (file_history | file_status | file_diff)` });
      }
    } catch (e) {
      reply(res, 200, { success: false, error: e instanceof Error ? e.message : String(e) });
    }
  });
});

server.on("error", (err) => {
  if (err.code === "EADDRINUSE") { console.error(`[git-mcp] :${PORT} in use — skipping`); return; }
  console.error("[git-mcp] server error:", err);
});

server.listen(PORT, "127.0.0.1", () => {
  console.error(`[git-mcp] /call on 127.0.0.1:${PORT} | history dir: ${HIST_DIR}`);
});
