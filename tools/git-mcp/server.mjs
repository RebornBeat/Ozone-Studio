// Ozone-Studio Git MCP — native git-like history over the SHARED history store (src/history, 2026-11-12).
//
// One store for BOTH kinds of thing: workspace FILES (scope "file", written by the file beacon) and ZSEI CONTAINER records (scope
// "container", written by the integrity monitor before every update/delete). Content is split into content-defined chunks stored once
// (history/objects/<hh>/<blake3>); each version is a row {v, ts, reason, path?, size, hash, chunks[]} in
// history/versions/<scope>/<id>.jsonl. A version is rebuilt by concatenating its chunks; differences are computed here on demand (Myers line
// diff; binary content reports changed chunks). Nothing is capped or pruned.
//
// Not git the binary (operator vision 2026-09-29): native to the graph, light, diff-based storage instead of full backups.
//
// Tools (POST /call {tool, input}):
//   history      {scope, id}                         → every version row of one item
//   status       {scope?}                            → every item with history: versions, last change, path/size
//   diff         {scope, id, from_v?, to_v?}         → hunks between two versions (defaults: previous → latest)
//   read_version {scope, id, v, encoding?}           → the content of a version ("utf8" default, or "base64")
//   restore      {scope, id, v}                      → asks the HOST to put the version back (operator action; host records the current state
//                                                      first so a restore is reversible). Needs `confirm: "RESTORE"`.
//   Legacy aliases (scope=file): file_history {container_id}, file_status {}, file_diff {container_id, from_ts?, to_ts?}
//
// Chunk and whole-content hashes are verified by the HOST on restore and by the integrity check; this reader does not re-hash (node has no
// blake3 built in) — a corrupt chunk is caught at restore.
//
// Env: OZONE_GIT_PORT (default 3235), OZONE_ZSEI_DATA_DIR, OZONE_HOST (default http://127.0.0.1:50051).

import { createServer } from "node:http";
import fs from "node:fs";
import path from "node:path";
import { guardOk } from "../shared/guard.mjs";

const PORT = Number(process.env.OZONE_GIT_PORT ?? 3235);
const DATA_DIR = process.env.OZONE_ZSEI_DATA_DIR
  ?? "/home/rebornbeat/Projects/Ozone-Studio/target/release/zsei_data";
const ROOT = path.join(DATA_DIR, "history");
const LEGACY_DIR = path.join(DATA_DIR, "file_history");
const TRACE_INT_GUARD = 60_000_000;
const HOST = (process.env.OZONE_HOST || "http://127.0.0.1:50051").replace(/\/+$/, "");

const SAFE = /^[A-Za-z0-9_-]{1,128}$/;
function safe(s, what) {
  const v = String(s ?? "");
  if (!SAFE.test(v)) throw new Error(`${what} must match [A-Za-z0-9_-]+`);
  return v;
}

function readRows(scope, id) {
  const p = path.join(ROOT, "versions", safe(scope, "scope"), `${safe(id, "id")}.jsonl`);
  if (!fs.existsSync(p)) return [];
  return fs.readFileSync(p, "utf8").split("\n").filter(Boolean)
    .map((l) => { try { return JSON.parse(l); } catch { return null; } })
    .filter(Boolean);
}

function chunkPath(hex) {
  if (!/^[0-9a-f]{3,}$/.test(hex)) throw new Error("malformed chunk id");
  return path.join(ROOT, "objects", hex.slice(0, 2), hex);
}

function rebuild(row) {
  const parts = row.chunks.map((c) => fs.readFileSync(chunkPath(c)));
  const buf = Buffer.concat(parts);
  if (buf.length !== row.size) throw new Error(`version ${row.v}: size ${buf.length} != recorded ${row.size} (corrupt or incomplete)`);
  return buf;
}

function rowOf(scope, id, v) {
  const rows = readRows(scope, id);
  if (!rows.length) throw new Error(`no history for ${scope}/${id}`);
  const r = rows.find((x) => x.v === Number(v));
  if (!r) throw new Error(`${scope}/${id} has no version ${v}`);
  return r;
}

function isBinary(buf) {
  const n = Math.min(buf.length, 8192);
  for (let i = 0; i < n; i++) if (buf[i] === 0) return true;
  return false;
}

// Myers O(ND) line diff on the region between the common prefix and suffix. Returns ops [{t:' '|'-'|'+', s}] for the whole input.
function diffLines(aLines, bLines) {
  let pre = 0;
  while (pre < aLines.length && pre < bLines.length && aLines[pre] === bLines[pre]) pre++;
  let suf = 0;
  while (suf < aLines.length - pre && suf < bLines.length - pre
    && aLines[aLines.length - 1 - suf] === bLines[bLines.length - 1 - suf]) suf++;
  const a = aLines.slice(pre, aLines.length - suf);
  const b = bLines.slice(pre, bLines.length - suf);
  const n = a.length; const m = b.length;
  const ops = [];
  let coarse = false;
  for (let i = 0; i < pre; i++) ops.push({ t: " ", s: aLines[i] });
  if (n === 0 || m === 0) {
    for (const s of a) ops.push({ t: "-", s });
    for (const s of b) ops.push({ t: "+", s });
  } else {
    const max = n + m;
    const off = max;
    let v = new Int32Array(2 * max + 2);
    const trace = [];
    let traceInts = 0;
    let found = false;
    for (let d = 0; d <= max && !found; d++) {
      // Host-memory guard (not a content cap): the trace grows ~D^2; past this many ints the middle region is reported as replaced
      // wholesale (coarse) instead of risking the process.
      traceInts += 2 * d + 3;
      if (traceInts > TRACE_INT_GUARD) { coarse = true; break; }
      trace.push({ d, arr: v.slice(off - d - 1, off + d + 2) });
      for (let k = -d; k <= d; k += 2) {
        let x;
        if (k === -d || (k !== d && v[off + k - 1] < v[off + k + 1])) x = v[off + k + 1];
        else x = v[off + k - 1] + 1;
        let y = x - k;
        while (x < n && y < m && a[x] === b[y]) { x++; y++; }
        v[off + k] = x;
        if (x >= n && y >= m) { found = true; break; }
      }
    }
    if (coarse) {
      for (const sl of a) ops.push({ t: "-", s: sl });
      for (const sl of b) ops.push({ t: "+", s: sl });
      for (let i = aLines.length - suf; i < aLines.length; i++) ops.push({ t: " ", s: aLines[i] });
      return ops;
    }
    // backtrack
    let x = n; let y = m;
    const rev = [];
    for (let d = trace.length - 1; d >= 0; d--) {
      const tr = trace[d];
      const vv = { at: (kk) => tr.arr[kk + d + 1] };
      const k = x - y;
      let pk;
      if (k === -d || (k !== d && vv.at(k - 1) < vv.at(k + 1))) pk = k + 1; else pk = k - 1;
      const px = vv.at(pk);
      const py = px - pk;
      while (x > px && y > py) { rev.push({ t: " ", s: a[x - 1] }); x--; y--; }
      if (d > 0) {
        if (x === px) { rev.push({ t: "+", s: b[y - 1] }); y--; } else { rev.push({ t: "-", s: a[x - 1] }); x--; }
      }
    }
    rev.reverse();
    ops.push(...rev);
  }
  for (let i = aLines.length - suf; i < aLines.length; i++) ops.push({ t: " ", s: aLines[i] });
  return ops;
}

// Group ops into unified hunks with 3 lines of context. EVERY changed line is kept (no cap).
function hunks(ops, ctx = 3) {
  const out = [];
  const changed = [];
  ops.forEach((o, i) => { if (o.t !== " ") changed.push(i); });
  if (!changed.length) return out;
  let start = Math.max(0, changed[0] - ctx);
  let end = Math.min(ops.length, changed[0] + ctx + 1);
  const groups = [];
  for (let c = 1; c < changed.length; c++) {
    const i = changed[c];
    if (i - ctx <= end) end = Math.min(ops.length, i + ctx + 1);
    else { groups.push([start, end]); start = Math.max(0, i - ctx); end = Math.min(ops.length, i + ctx + 1); }
  }
  groups.push([start, end]);
  for (const [s, e] of groups) {
    let aStart = 0; let bStart = 0;
    for (let i = 0; i < s; i++) { if (ops[i].t !== "+") aStart++; if (ops[i].t !== "-") bStart++; }
    let aLen = 0; let bLen = 0;
    const lines = [];
    for (let i = s; i < e; i++) {
      if (ops[i].t !== "+") aLen++;
      if (ops[i].t !== "-") bLen++;
      lines.push(`${ops[i].t}${ops[i].s}`);
    }
    out.push({ header: `@@ -${aStart + 1},${aLen} +${bStart + 1},${bLen} @@`, lines });
  }
  return out;
}

function diffVersions(scope, id, fromV, toV) {
  const rows = readRows(scope, id);
  if (rows.length < 2 && fromV == null && toV == null) {
    return { diff: [], note: rows.length ? "only one version recorded" : "no history" };
  }
  const to = toV != null ? rows.find((r) => r.v === Number(toV)) : rows.at(-1);
  const from = fromV != null ? rows.find((r) => r.v === Number(fromV)) : rows.at(-2);
  if (!from || !to) throw new Error("version not found");
  if (from.v === to.v) return { diff: [], note: "same version" };
  const a = rebuild(from); const b = rebuild(to);
  const meta = { from: { v: from.v, ts: from.ts, size: from.size, reason: from.reason }, to: { v: to.v, ts: to.ts, size: to.size, reason: to.reason } };
  if (isBinary(a) || isBinary(b)) {
    const sa = new Set(from.chunks); const sb = new Set(to.chunks);
    return {
      ...meta,
      binary: true,
      chunks_from: from.chunks.length,
      chunks_to: to.chunks.length,
      chunks_shared: to.chunks.filter((c) => sa.has(c)).length,
      chunks_added: to.chunks.filter((c) => !sa.has(c)).length,
      chunks_removed: from.chunks.filter((c) => !sb.has(c)).length,
    };
  }
  const ops = diffLines(a.toString("utf8").split("\n"), b.toString("utf8").split("\n"));
  return { ...meta, hunks: hunks(ops) };
}

function status(scopeFilter) {
  const out = [];
  const vdir = path.join(ROOT, "versions");
  if (fs.existsSync(vdir)) {
    for (const scope of fs.readdirSync(vdir)) {
      if (scopeFilter && scope !== scopeFilter) continue;
      for (const f of fs.readdirSync(path.join(vdir, scope)).filter((x) => x.endsWith(".jsonl"))) {
        const id = f.replace(/\.jsonl$/, "");
        const rows = readRows(scope, id);
        if (!rows.length) continue;
        const last = rows.at(-1);
        out.push({ scope, id, versions: rows.length, last_ts: last.ts, size: last.size, path: [...rows].reverse().find((r) => r.path)?.path ?? null });
      }
    }
  }
  return out.sort((x, y) => y.last_ts - x.last_ts);
}

async function hostRestore(scope, id, v, confirm) {
  if (confirm !== "RESTORE") throw new Error('restore is an operator action: pass confirm:"RESTORE"');
  const r = await fetch(`${HOST}/history/restore`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ scope, id, version: Number(v), confirm: "RESTORE", by: "git-mcp" }),
  });
  return r.json();
}

function reply(res, code, body) {
  res.writeHead(code, { "Content-Type": "application/json" });
  res.end(JSON.stringify(body));
}

async function handle(tool, input) {
  if (tool === "history" || tool === "file_history") {
    const scope = tool === "file_history" ? "file" : safe(input.scope, "scope");
    const id = safe(tool === "file_history" ? input.container_id : input.id, "id");
    const rows = readRows(scope, id);
    return { scope, id, versions: rows.length, history: rows };
  }
  if (tool === "status" || tool === "file_status") {
    const scope = tool === "file_status" ? "file" : (input.scope ? safe(input.scope, "scope") : null);
    return { watched: status(scope) };
  }
  if (tool === "diff") {
    return diffVersions(safe(input.scope, "scope"), safe(input.id, "id"), input.from_v, input.to_v);
  }
  if (tool === "file_diff") {
    // legacy shape: timestamps instead of version numbers
    const id = safe(input.container_id, "id");
    const rows = readRows("file", id);
    const fromTs = Number(input.from_ts ?? 0);
    const toTs = Number(input.to_ts ?? Date.now() + 1e9);
    const at = (ts) => [...rows].filter((r) => r.ts <= ts).at(-1);
    const from = input.from_ts == null ? rows.at(-2) : at(fromTs);
    const to = at(toTs);
    if (!from || !to || from.v === to.v) return { diff: [], note: "no distinct version pair in range" };
    return diffVersions("file", id, from.v, to.v);
  }
  if (tool === "read_version") {
    const scope = safe(input.scope, "scope"); const id = safe(input.id, "id");
    const row = rowOf(scope, id, input.v);
    const buf = rebuild(row);
    return { scope, id, v: row.v, size: row.size, encoding: input.encoding === "base64" ? "base64" : "utf8",
      content: input.encoding === "base64" ? buf.toString("base64") : buf.toString("utf8") };
  }
  if (tool === "restore") {
    const scope = safe(input.scope, "scope"); const id = safe(input.id, "id");
    rowOf(scope, id, input.v); // the version must exist
    return await hostRestore(scope, id, input.v, input.confirm);
  }
  throw new Error(`unknown git tool '${tool}' (history | status | diff | read_version | restore | file_history | file_status | file_diff)`);
}

const server = createServer((req, res) => {
  if (!guardOk(req, res)) return; // loopback guard (tools/shared/guard.mjs)
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
    try {
      reply(res, 200, { success: true, output: await handle(tool, input) });
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
  console.error(`[git-mcp] /call on 127.0.0.1:${PORT} | history root: ${ROOT} (legacy: ${LEGACY_DIR})`);
});
