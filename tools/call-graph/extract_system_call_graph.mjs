#!/usr/bin/env node
// Extract Ozone-Studio's OWN call graphs from the source and registries into one machine-readable asset:
//   assets/call_graphs/system_call_graph.json
// The host reads that asset at boot (src/orchestrator/call_graph.rs) and materialises one graph container per graph (idempotent), and
// GET /callgraph/system serves it to the UI. Nothing here runs the system, calls a model or touches a data dir: it READS source + docs.
//
// Graphs produced:
//   walk   — the orchestration walk: stage functions in the order execute_stages calls them (order edges), the functions each stage
//            reaches that make a model call (reaches edges, with the shortest `via` path), and the model-call sites themselves
//            (call_site labels passed to metered_execute_resilient; strength-N yes/no vote sites have no label and are listed as vote sites).
//   loops  — background loops: every labelled capture_loop_model_call site grouped under the file (loop) that makes it.
//   routes — docs/CALL_ROUTE_REGISTRY.md: every route (R*) and the guardrail layers (G*) it names (guarded_by edges).
//
// STATIC and NAME-BASED by design: reachability follows `self.name(` / `Self::name(` / `name(` calls to functions whose name is defined
// exactly once under src/orchestrator; a same-named function defined twice is left out (and counted in `notes`) rather than guessed.
// No confidence, no scores: an edge is present (found in the source) or absent.
//
// Usage:  node tools/call-graph/extract_system_call_graph.mjs            write the asset
//         node tools/call-graph/extract_system_call_graph.mjs --check    exit 1 if the asset on disk differs from a fresh extraction
//         node tools/call-graph/extract_system_call_graph.mjs --stdout   print instead of writing
import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const OUT = path.join(ROOT, "assets/call_graphs/system_call_graph.json");
const ORCH = path.join(ROOT, "src/orchestrator");
const read = (p) => fs.readFileSync(p, "utf8");
const rel = (p) => path.relative(ROOT, p).split(path.sep).join("/");

// ── source files (non-test part only) ─────────────────────────────────────────────────────────────────────────
function sourceFiles() {
  const out = [];
  const walk = (d) => {
    for (const e of fs.readdirSync(d, { withFileTypes: true })) {
      const p = path.join(d, e.name);
      if (e.isDirectory()) walk(p);
      else if (e.name.endsWith(".rs")) out.push(p);
    }
  };
  walk(ORCH);
  out.push(path.join(ROOT, "src/consciousness/assistant.rs"));
  return out.filter((p) => fs.existsSync(p)).sort();
}

function nonTestLines(text) {
  const lines = text.split("\n");
  const cut = lines.findIndex((l) => /^\s*#\[cfg\(test\)\]/.test(l));
  return cut === -1 ? lines : lines.slice(0, cut);
}

const FN_RE = /^(\s*)(?:pub(?:\([a-z]+\))?\s+)?(?:async\s+)?(?:unsafe\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)/;

const files = sourceFiles();
const parsed = files.map((f) => ({ file: f, rel: rel(f), lines: nonTestLines(read(f)) }));

// Brace matcher: index of the `}` closing the first `{` at/after `from`, skipping strings, raw strings, char literals and comments.
// Returns -1 for a declaration with no body (a trait method ending in `;` before any `{`).
function matchBody(text, from) {
  let i = from;
  const n = text.length;
  let depth = 0;
  let started = false;
  while (i < n) {
    const c = text[i];
    const c2 = text.slice(i, i + 2);
    if (c2 === "//") { while (i < n && text[i] !== "\n") i++; continue; }
    if (c2 === "/*") { const j = text.indexOf("*/", i + 2); i = j === -1 ? n : j + 2; continue; }
    if (c === "r" && /^r#*"/.test(text.slice(i, i + 10)) && !/[A-Za-z0-9_]/.test(text[i - 1] ?? " ")) {
      const hashes = text.slice(i + 1).match(/^#*/)[0];
      const close = '"' + hashes;
      const j = text.indexOf(close, i + 2 + hashes.length);
      i = j === -1 ? n : j + close.length;
      continue;
    }
    if (c === '"') { i++; while (i < n && text[i] !== '"') { if (text[i] === "\\") i++; i++; } i++; continue; }
    if (c === "'") {
      // char literal ('x', '\n', '\u{..}') — a lifetime ('a) has no closing quote within 4 chars
      const m = text.slice(i, i + 12).match(/^'(?:\\u\{[0-9a-fA-F]+\}|\\.|[^\\'])'/);
      if (m) { i += m[0].length; continue; }
      i++; continue;
    }
    if (c === ";" && !started) return -1;
    if (c === "{") { depth++; started = true; }
    else if (c === "}") { depth--; if (started && depth === 0) return i; }
    i++;
  }
  return -1;
}

// function index: name -> [{file, line, endLine, indent}]
const fnDefs = new Map();
for (const pf of parsed) {
  const text = pf.lines.join("\n");
  const lineStarts = [0];
  for (let i = 0; i < text.length; i++) if (text[i] === "\n") lineStarts.push(i + 1);
  const lineOf = (pos) => {
    let lo = 0, hi = lineStarts.length - 1;
    while (lo < hi) { const mid = (lo + hi + 1) >> 1; if (lineStarts[mid] <= pos) lo = mid; else hi = mid - 1; }
    return lo + 1;
  };
  pf.lines.forEach((l, i) => {
    const m = l.match(FN_RE);
    if (!m) return;
    const end = matchBody(text, lineStarts[i]);
    if (end === -1) return; // no body (trait declaration)
    const d = { name: m[2], line: i + 1, indent: m[1].length, endLine: lineOf(end), file: pf.rel };
    if (!fnDefs.has(d.name)) fnDefs.set(d.name, []);
    fnDefs.get(d.name).push(d);
  });
}
const unique = new Map(); // names defined exactly once
let ambiguous = 0;
for (const [n, defs] of fnDefs) {
  if (defs.length === 1) unique.set(n, defs[0]);
  else ambiguous++;
}
const bodyOf = (def) => parsed.find((p) => p.rel === def.file).lines.slice(def.line - 1, def.endLine).join("\n");

// ── model-call sites ───────────────────────────────────────────────────────────────────────────────────────────
// A site = a labelled call. `fn` is the enclosing function (nearest preceding fn at the smallest indent that covers the line).
const enclosing = (pf, line) => {
  let best = null;
  for (const defs of fnDefs.values()) {
    for (const d of defs) {
      if (d.file === pf.rel && d.line <= line && line <= d.endLine && (!best || d.line > best.line)) best = d;
    }
  }
  return best;
};

const sites = new Map(); // label -> {label, kind, file, line, fn}
const voteFns = new Map(); // fn name -> {file,line}
const SITE_RES = [
  { kind: "model_call", re: /metered_execute_resilient(?:_raw)?\s*\(\s*[^;"]*?,\s*"([A-Za-z0-9_:.\-]+)"\s*\)/g },
  { kind: "loop_call", re: /capture_loop_model_call(?:_with)?\s*\(\s*"([A-Za-z0-9_:.\-]+)"/g },
];
for (const pf of parsed) {
  const text = pf.lines.join("\n");
  for (const { kind, re } of SITE_RES) {
    re.lastIndex = 0;
    let m;
    while ((m = re.exec(text))) {
      const line = text.slice(0, m.index).split("\n").length;
      const fnd = enclosing(pf, line);
      const label = m[1];
      if (!sites.has(label)) sites.set(label, { label, kind, file: pf.rel, line, fn: fnd ? fnd.name : null });
    }
  }
  pf.lines.forEach((l, i) => {
    if (/\bconfirm_yes_no(?:_batch|_orch|_tokens)?\s*\(/.test(l) && !FN_RE.test(l)) {
      const fnd = enclosing(pf, i + 1);
      if (fnd && !voteFns.has(fnd.name)) voteFns.set(fnd.name, { file: pf.rel, line: i + 1 });
    }
  });
}

// ── caller graph among unique fns ──────────────────────────────────────────────────────────────────────────────
// the walk's entry points are never a CALLEE of a stage (a stage must not "reach" the driver that calls it)
const ENTRY_FNS = new Set(["orchestrate", "orchestrate_traced", "execute_stages"]);
const callees = new Map(); // name -> Set(name)
for (const [name, def] of unique) {
  // comments stripped; only `self.name(`, `Self::name(` and a bare `name(` count — NEVER `x.name(` (a method on another type that merely
  // shares a name with one of ours, e.g. `set.union(..)`)
  const body = bodyOf(def).split("\n").slice(1).map((l) => l.replace(/\/\/.*$/, "")).join("\n");
  const set = new Set();
  const re = /(?<![.\w])(?:self\.|Self::)?([a-z_][a-z0-9_]{4,})\s*\(/g;
  let m;
  while ((m = re.exec(body))) {
    const c = m[1];
    if (c !== name && unique.has(c) && !ENTRY_FNS.has(c)) set.add(c);
  }
  callees.set(name, set);
}

// BFS from `start`: shortest path to every reachable fn
function reach(start) {
  const prev = new Map([[start, null]]);
  const q = [start];
  while (q.length) {
    const cur = q.shift();
    for (const n of [...(callees.get(cur) ?? [])].sort()) {
      if (!prev.has(n)) { prev.set(n, cur); q.push(n); }
    }
  }
  const pathTo = (t) => {
    const p = [];
    for (let c = t; c !== null; c = prev.get(c)) p.unshift(c);
    return p;
  };
  return { prev, pathTo };
}

// ── walk graph ──────────────────────────────────────────────────────────────────────────────────────────────────
const driver = unique.get("execute_stages");
const nodesWalk = [];
const edgesWalk = [];
const nodeIds = new Set();
const addNode = (arr, n) => { if (!nodeIds.has(n.id)) { nodeIds.add(n.id); arr.push(n); } };

const siteFns = new Map(); // fn name -> [site labels] for the walk (model_call + vote)
for (const s of sites.values()) {
  if (s.kind === "model_call" && s.fn) {
    if (!siteFns.has(s.fn)) siteFns.set(s.fn, []);
    siteFns.get(s.fn).push(s.label);
  }
}

const registryText = fs.existsSync(path.join(ROOT, "docs/ZERO_SHOT_CALL_REGISTRY.md")) ? read(path.join(ROOT, "docs/ZERO_SHOT_CALL_REGISTRY.md")) : "";
const inRegistry = (label) => registryText.includes(label);

if (driver) {
  const body = bodyOf(driver).split("\n");
  const order = [];
  body.forEach((l, i) => {
    const re = /self\.([a-z_][a-z0-9_]{3,})\s*\(/g;
    let m;
    while ((m = re.exec(l))) {
      const name = m[1];
      if (!unique.has(name) || name === "execute_stages" || order.includes(name)) continue;
      const { prev } = reach(name);
      const hasSite = [...prev.keys()].some((k) => siteFns.has(k) || voteFns.has(k));
      if (/^record_|^capture_/.test(name)) continue; // bookkeeping helpers, not walk stages
      if (/^stage_/.test(name) || hasSite) order.push(name);
    }
  });
  let prevId = null;
  order.forEach((name, idx) => {
    const d = unique.get(name);
    const id = `stage:${name}`;
    addNode(nodesWalk, { id, kind: "stage", label: name, order: idx + 1, file: d.file, line: d.line });
    if (prevId) edgesWalk.push({ from: prevId, to: id, kind: "order" });
    prevId = id;
    const { prev, pathTo } = reach(name);
    for (const k of [...prev.keys()].sort()) {
      const labels = siteFns.get(k) ?? [];
      const isVote = voteFns.has(k);
      if (!labels.length && !isVote) continue;
      const fd = unique.get(k);
      const fid = `fn:${k}`;
      addNode(nodesWalk, { id: fid, kind: "fn", label: k, file: fd.file, line: fd.line });
      if (k !== name) edgesWalk.push({ from: id, to: fid, kind: "reaches", via: pathTo(k) });
      else edgesWalk.push({ from: id, to: fid, kind: "reaches", via: [name] });
      for (const lab of labels) {
        const s = sites.get(lab);
        const sid = `site:${lab}`;
        addNode(nodesWalk, { id: sid, kind: "call_site", label: lab, file: s.file, line: s.line, in_registry: inRegistry(lab) });
        if (!edgesWalk.some((e) => e.from === fid && e.to === sid)) edgesWalk.push({ from: fid, to: sid, kind: "calls_model" });
      }
      if (isVote) {
        const v = voteFns.get(k);
        const sid = `vote:${k}`;
        addNode(nodesWalk, { id: sid, kind: "vote_site", label: `strength-N yes/no votes in ${k}`, file: v.file, line: v.line });
        edgesWalk.push({ from: fid, to: sid, kind: "calls_vote" });
      }
    }
  });
}

// A model-call site no stage reaches by name (reached through a closure, a trait object or an ambiguous name) is KEPT, marked unattached —
// never silently dropped.
for (const s of [...sites.values()].filter((x) => x.kind === "model_call").sort((a, b) => a.label.localeCompare(b.label))) {
  const sid = `site:${s.label}`;
  if (!nodeIds.has(sid)) addNode(nodesWalk, { id: sid, kind: "call_site", label: s.label, file: s.file, line: s.line, in_registry: inRegistry(s.label), attached: false, fn: s.fn });
}

// ── loops graph ─────────────────────────────────────────────────────────────────────────────────────────────────
const nodesLoops = [];
const edgesLoops = [];
const loopNodeIds = new Set();
for (const s of [...sites.values()].filter((x) => x.kind === "loop_call").sort((a, b) => a.label.localeCompare(b.label))) {
  const stem = path.basename(s.file, ".rs");
  const lid = `loop:${stem}`;
  if (!loopNodeIds.has(lid)) { loopNodeIds.add(lid); nodesLoops.push({ id: lid, kind: "loop", label: stem, file: s.file }); }
  const sid = `site:${s.label}`;
  if (!loopNodeIds.has(sid)) {
    loopNodeIds.add(sid);
    nodesLoops.push({ id: sid, kind: "call_site", label: s.label, file: s.file, line: s.line, in_registry: inRegistry(s.label) });
  }
  edgesLoops.push({ from: lid, to: sid, kind: "calls_model" });
}

// ── routes graph (docs/CALL_ROUTE_REGISTRY.md) ─────────────────────────────────────────────────────────────────
const nodesRoutes = [];
const edgesRoutes = [];
const routeDoc = path.join(ROOT, "docs/CALL_ROUTE_REGISTRY.md");
if (fs.existsSync(routeDoc)) {
  const cells = (l) => l.split("|").slice(1, -1).map((c) => c.trim());
  const strip = (s) => s.replace(/\*\*|`/g, "");
  const seen = new Set();
  for (const l of read(routeDoc).split("\n")) {
    if (!l.startsWith("|")) continue;
    const c = cells(l);
    const g = c.length >= 3 ? c[0].match(/^(G\d+)\s+(.+)$/) : null;
    if (g && !seen.has(g[1])) {
      seen.add(g[1]);
      nodesRoutes.push({ id: `guard:${g[1]}`, kind: "guardrail", label: `${g[1]} ${strip(g[2])}`, where: strip(c[1]), detail: strip(c[2]) });
    }
  }
  for (const l of read(routeDoc).split("\n")) {
    if (!l.startsWith("|")) continue;
    const c = cells(l);
    if (c.length >= 4 && /^R\d+$/.test(c[0]) && !seen.has(c[0])) {
      seen.add(c[0]);
      const wide = c.length >= 6; // R1–R12: # | route | entry | guardrails | capture sink | status ; R13+: # | route | guardrails | what
      nodesRoutes.push({ id: `route:${c[0]}`, kind: "route", label: `${c[0]} ${strip(c[1])}`, ...(wide ? { entry: strip(c[2]), status: strip(c[5]) } : { detail: strip(c[3]) }) });
      for (const g of new Set(strip(wide ? c[3] : c[2]).match(/\bG\d+\b/g) ?? [])) {
        if (nodesRoutes.some((n) => n.id === `guard:${g}`)) edgesRoutes.push({ from: `route:${c[0]}`, to: `guard:${g}`, kind: "guarded_by" });
      }
    }
  }
}

// ── assemble (deterministic: no timestamps) ─────────────────────────────────────────────────────────────────────
const inputs = [...files.map(rel), "docs/CALL_ROUTE_REGISTRY.md", "docs/ZERO_SHOT_CALL_REGISTRY.md"].filter((p) => fs.existsSync(path.join(ROOT, p)));
const inputsHash = crypto.createHash("sha256");
for (const p of inputs) inputsHash.update(p).update(fs.readFileSync(path.join(ROOT, p)));

const doc = {
  schema: 1,
  generator: "tools/call-graph/extract_system_call_graph.mjs",
  inputs_sha256: inputsHash.digest("hex"),
  notes: [
    "Static and name-based: reachability follows calls to functions whose name is defined exactly once under src/orchestrator; same-named functions are left out, never guessed.",
    `${ambiguous} function name(s) are defined more than once and take no part in reachability.`,
    "An edge is present (found in the source) or absent. Nothing here is scored.",
    "call_site.in_registry = the label appears in docs/ZERO_SHOT_CALL_REGISTRY.md (a call site with no row there is unregistered).",
  ],
  graphs: [
    { key: "walk", title: "Orchestration walk — stages, the functions that make model calls, and the call sites", nodes: nodesWalk, edges: edgesWalk },
    { key: "loops", title: "Background loops — labelled model-call sites per loop", nodes: nodesLoops, edges: edgesLoops },
    { key: "routes", title: "Call routes and the guardrail layers on each (docs/CALL_ROUTE_REGISTRY.md)", nodes: nodesRoutes, edges: edgesRoutes },
  ],
};
const text = JSON.stringify(doc, null, 2) + "\n";

if (process.argv.includes("--stdout")) {
  process.stdout.write(text);
} else if (process.argv.includes("--check")) {
  const cur = fs.existsSync(OUT) ? fs.readFileSync(OUT, "utf8") : "";
  if (cur !== text) { console.error("assets/call_graphs/system_call_graph.json is out of date — run the extractor"); process.exit(1); }
  console.log("call graph asset is current");
} else {
  fs.mkdirSync(path.dirname(OUT), { recursive: true });
  fs.writeFileSync(OUT, text);
  const c = (g) => `${g.nodes.length} nodes / ${g.edges.length} edges`;
  console.log(`wrote ${rel(OUT)}: ` + doc.graphs.map((g) => `${g.key} ${c(g)}`).join("; "));
}
